//! The three sheet actions: request -> messages, answer -> [`Outcome`].
//!
//! The window applies an outcome through the engine's own commands inside one undo group, so the
//! AI never writes the `.xlsx` directly and a failed apply rolls back. The model's answer is read by
//! the shared tolerant extractor ([`crate::extract_json`]); the structure (column count, row width,
//! scalar cells) is checked here, so a bad answer is a message, not a half-written sheet.

use local_ai::Msg;
use serde::Deserialize;
use serde_json::Value;

use crate::extract_json;
use crate::prompts::{ENGINE_FUNCTIONS, fill, for_lang};

/// What the user asked for.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// A free description -> a header and `rows` data rows, on a new sheet.
    GenerateSheet { topic: String, rows: u32 },
    /// The current column plus its neighbours -> `count` values down the column.
    FillColumn { header: String, context: String, count: u32 },
    /// A question about a cell plus the table around it -> one formula.
    Formula { question: String, cell: String, context: String },
}

/// One cell of an answer: a plain value, or a formula the spreadsheet computes.
///
/// Text is **never** read as a formula (a value that starts with `=` is written as text), and a
/// computed cell is **never** written as an invented number: it is a formula in `f`, which the
/// window validates with the engine before applying.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    /// A scalar written as typed.
    Value(Value),
    /// `=...`, as the model wrote it (a missing `=` is added).
    Formula(String),
}

/// What the answer became, ready to apply with engine commands.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// A new sheet: `columns` is the header row, `rows` the data.
    Sheet { title: String, columns: Vec<String>, rows: Vec<Vec<Cell>> },
    /// Values for a column (top to bottom).
    Values { values: Vec<Cell> },
    /// One formula, starting with `=`.
    Formula { formula: String },
}

impl Request {
    /// The short label the status strip shows while the worker runs (translated by the window).
    pub fn label(&self) -> &'static str {
        match self {
            Request::GenerateSheet { .. } => "Generating a sheet",
            Request::FillColumn { .. } => "Filling the column",
            Request::Formula { .. } => "Writing the formula",
        }
    }

    /// The subject appended to the label (the topic, the column header, the cell).
    pub fn subject(&self) -> &str {
        match self {
            Request::GenerateSheet { topic, .. } => topic,
            Request::FillColumn { header, .. } => header,
            Request::Formula { cell, .. } => cell,
        }
    }
}

/// The conversation one request sends: a system turn and one user turn.
pub fn messages(request: &Request, lang: &str) -> Result<Vec<Msg>, String> {
    let prompts = for_lang(lang);
    let system = Msg::System(fill(prompts.system, &[("answer_in", prompts.answer_in)]));
    let user = match request {
        Request::GenerateSheet { topic, rows } => {
            let topic = topic.trim();
            if topic.is_empty() {
                return Err("the topic is empty".into());
            }
            fill(
                prompts.generate,
                &[("topic", topic), ("rows", &rows.to_string()), ("answer_in", prompts.answer_in), ("functions", ENGINE_FUNCTIONS)],
            )
        }
        Request::FillColumn { header, context, count } => fill(
            prompts.fill,
            &[
                ("header", header.trim()),
                ("context", context),
                ("count", &count.to_string()),
                ("answer_in", prompts.answer_in),
                ("functions", ENGINE_FUNCTIONS),
            ],
        ),
        Request::Formula { question, cell, context } => {
            let question = question.trim();
            if question.is_empty() {
                return Err("the question is empty".into());
            }
            fill(prompts.formula, &[("cell", cell), ("question", question), ("context", context), ("answer_in", prompts.answer_in)])
        }
    };
    Ok(vec![system, Msg::User(user)])
}

/// Turns one answer into what the window applies. Structural problems are errors with the row that
/// failed; the requested count is a target, so a model that answers a few rows more or fewer is
/// accepted (the window applies what came).
pub fn interpret(request: &Request, answer: &str) -> Result<Outcome, String> {
    match request {
        Request::GenerateSheet { topic, rows } => {
            let requested = *rows as usize;
            let sheet: SheetAnswer = extract_json(answer)?;
            if sheet.columns.is_empty() {
                return Err("the model did not answer any column".into());
            }
            if sheet.rows.is_empty() {
                return Err("the model did not answer any data row".into());
            }
            let width = sheet.columns.len();
            for (index, row) in sheet.rows.iter().enumerate() {
                if row.len() != width {
                    return Err(format!("the model answered row {} with {} values, expected {width}", index + 1, row.len()));
                }
            }
            let mut data: Vec<Vec<Cell>> = Vec::with_capacity(sheet.rows.len());
            for (row_index, row) in sheet.rows.into_iter().enumerate() {
                let mut line = Vec::with_capacity(width);
                for (col_index, value) in row.into_iter().enumerate() {
                    line.push(cell_of(value).map_err(|e| format!("row {} column {}: {e}", row_index + 1, col_index + 1))?);
                }
                data.push(line);
            }
            data.truncate(requested.max(1));
            let title =
                if sheet.title.trim().is_empty() { topic.trim().chars().take(40).collect() } else { sheet.title.trim().chars().take(40).collect() };
            Ok(Outcome::Sheet { title, columns: sheet.columns, rows: data })
        }
        Request::FillColumn { count, .. } => {
            let answer: ValuesAnswer = extract_json(answer)?;
            if answer.values.is_empty() {
                return Err("the model did not answer any value".into());
            }
            let mut values: Vec<Cell> = Vec::with_capacity(answer.values.len());
            for (index, value) in answer.values.into_iter().enumerate() {
                values.push(cell_of(value).map_err(|e| format!("value {}: {e}", index + 1))?);
            }
            values.truncate(*count as usize);
            Ok(Outcome::Values { values })
        }
        Request::Formula { .. } => {
            let answer: FormulaAnswer = extract_json(answer)?;
            let formula = answer.formula.trim().lines().next().unwrap_or("").trim();
            if formula.is_empty() {
                return Err("the model did not answer a formula".into());
            }
            Ok(Outcome::Formula { formula: if formula.starts_with('=') { formula.to_owned() } else { format!("={formula}") } })
        }
    }
}

/// A cell value the engine's `range.setValues` accepts: a JSON number, string, boolean or null.
fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_))
}

/// One cell as the contract allows it: a JSON scalar, `{"v": value}` (written as typed) or
/// `{"f": "=A2+B2"}` (a formula the engine evaluates). Anything else is refused, so a model cannot
/// smuggle a nested value into a cell.
fn cell_of(value: Value) -> Result<Cell, String> {
    let Value::Object(mut fields) = value else {
        return if is_scalar(&value) { Ok(Cell::Value(value)) } else { Err("the cell is a nested value".into()) };
    };
    if let Some(formula) = fields.remove("f") {
        let Some(text) = formula.as_str() else { return Err("the cell's `f` is not text".into()) };
        let formula = text.trim();
        if formula.is_empty() {
            return Err("the cell's `f` is empty".into());
        }
        return Ok(Cell::Formula(if formula.starts_with('=') { formula.to_string() } else { format!("={formula}") }));
    }
    if let Some(value) = fields.remove("v") {
        return if is_scalar(&value) { Ok(Cell::Value(value)) } else { Err("the cell's `v` is a nested value".into()) };
    }
    Err("the cell object has neither `v` nor `f`".into())
}

#[derive(Deserialize)]
struct SheetAnswer {
    #[serde(default)]
    title: String,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    rows: Vec<Vec<Value>>,
}

#[derive(Deserialize)]
struct ValuesAnswer {
    #[serde(default)]
    values: Vec<Value>,
}

#[derive(Deserialize)]
struct FormulaAnswer {
    #[serde(default)]
    formula: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate() -> Request {
        Request::GenerateSheet { topic: "medicamentos".into(), rows: 3 }
    }

    #[test]
    fn a_sheet_answer_becomes_columns_and_rows() {
        let answer = r#"{"title":"Medicamentos","columns":["Nome","Dose"],"rows":[["A",1],["B",2]]}"#;
        let outcome = interpret(&generate(), answer).expect("a sheet");
        match outcome {
            Outcome::Sheet { title, columns, rows } => {
                assert_eq!(title, "Medicamentos");
                assert_eq!(columns, ["Nome", "Dose"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0], vec![Cell::Value(Value::String("A".into())), Cell::Value(Value::from(1))]);
            }
            other => panic!("expected a sheet, got {other:?}"),
        }
    }

    /// The computed cells the model is now asked for: `{"f": ...}` becomes a formula, and the text
    /// it is written as is exactly what the engine will parse.
    #[test]
    fn a_formula_cell_becomes_a_formula() {
        let answer = r#"{"columns":["A","B","Soma"],"rows":[[2,3,{"f":"=A2+B2"}]]}"#;
        match interpret(&generate(), answer).expect("a sheet") {
            Outcome::Sheet { rows, .. } => {
                assert_eq!(rows[0][0], Cell::Value(Value::from(2)));
                assert_eq!(rows[0][2], Cell::Formula("=A2+B2".into()));
            }
            other => panic!("{other:?}"),
        }
    }

    /// `{"v": ...}` is a plain value, and a `v` that starts with `=` stays **text**: the anti-sorcery
    /// rule is unchanged, only the formula channel is new.
    #[test]
    fn a_plain_value_stays_text_even_with_a_leading_equals() {
        let answer = r#"{"columns":["A"],"rows":[[{"v":"=A2+B2"}]]}"#;
        match interpret(&generate(), answer).expect("a sheet") {
            Outcome::Sheet { rows, .. } => assert_eq!(rows[0][0], Cell::Value(Value::String("=A2+B2".into()))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_formula_cell_without_the_equals_gets_it() {
        let answer = r#"{"columns":["A"],"rows":[[{"f":"A2+1"}]]}"#;
        match interpret(&generate(), answer).expect("a sheet") {
            Outcome::Sheet { rows, .. } => assert_eq!(rows[0][0], Cell::Formula("=A2+1".into())),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_cell_object_with_neither_key_is_refused_with_its_place() {
        let answer = r#"{"columns":["A"],"rows":[[{"x":1}]]}"#;
        let error = interpret(&generate(), answer).expect_err("no v/f");
        assert!(error.contains("row 1 column 1") && error.contains("`v`"), "{error}");
    }

    /// A reasoning block and a markdown fence are the extractor's problem, not this action's.
    #[test]
    fn the_extractor_reads_through_reasoning_and_fences() {
        let answer = "<think>{not the answer}</think>\n```json\n{\"columns\":[\"A\"],\"rows\":[[\"x\"]]}\n```";
        let outcome = interpret(&generate(), answer).expect("after think");
        assert!(matches!(outcome, Outcome::Sheet { .. }));
    }

    /// A row with the wrong width is a structural error that names the row.
    #[test]
    fn a_row_with_the_wrong_width_is_refused() {
        let answer = r#"{"columns":["A","B"],"rows":[["x"]]}"#;
        let error = interpret(&generate(), answer).expect_err("wrong width");
        assert!(error.contains("row 1") && error.contains("expected 2"), "{error}");
    }

    #[test]
    fn a_nested_cell_value_is_refused() {
        let answer = r#"{"columns":["A"],"rows":[[{"v":[1,2]}]]}"#;
        let error = interpret(&generate(), answer).expect_err("nested");
        assert!(error.contains("nested"), "{error}");
    }

    #[test]
    fn the_title_defaults_to_the_topic() {
        let answer = r#"{"columns":["A"],"rows":[[1]]}"#;
        match interpret(&generate(), answer).expect("sheet") {
            Outcome::Sheet { title, .. } => assert_eq!(title, "medicamentos"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fill_values_are_taken_in_order_and_capped_at_the_count() {
        let request = Request::FillColumn { header: "Dose".into(), context: String::new(), count: 2 };
        let answer = r#"{"values":["1 mg",{"f":"=B2*2"},"3 mg"]}"#;
        match interpret(&request, answer).expect("values") {
            Outcome::Values { values } => {
                assert_eq!(values, vec![Cell::Value(Value::String("1 mg".into())), Cell::Formula("=B2*2".into())]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_formula_without_the_equals_gets_it() {
        let request = Request::Formula { question: "total".into(), cell: "B2".into(), context: String::new() };
        match interpret(&request, r#"{"formula":"SUM(A1:A3)"}"#).expect("formula") {
            Outcome::Formula { formula } => assert_eq!(formula, "=SUM(A1:A3)"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_empty_topic_is_refused_before_the_request() {
        let request = Request::GenerateSheet { topic: "  ".into(), rows: 3 };
        assert!(messages(&request, "en").is_err());
    }

    #[test]
    fn the_messages_carry_the_contract_in_the_language() {
        let messages = messages(&generate(), "pt-br").expect("messages");
        assert_eq!(messages.len(), 2);
        let user = match &messages[1] {
            Msg::User(text) => text,
            other => panic!("{other:?}"),
        };
        assert!(user.contains("\"columns\"") && user.contains("medicamentos"));
        assert!(user.contains("português"), "answer in the user's language: {user}");
    }
}
