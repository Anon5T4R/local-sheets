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
use crate::prompts::{fill, for_lang};

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

/// What the answer became, ready to apply with engine commands.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// A new sheet: `columns` is the header row, `rows` the data.
    Sheet { title: String, columns: Vec<String>, rows: Vec<Vec<Value>> },
    /// Values for a column (top to bottom).
    Values { values: Vec<Value> },
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
            fill(prompts.generate, &[("topic", topic), ("rows", &rows.to_string()), ("answer_in", prompts.answer_in)])
        }
        Request::FillColumn { header, context, count } => {
            fill(prompts.fill, &[("header", header.trim()), ("context", context), ("count", &count.to_string()), ("answer_in", prompts.answer_in)])
        }
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
                if let Some(bad) = row.iter().position(|value| !is_scalar(value)) {
                    return Err(format!("the model answered row {} column {} with a nested value", index + 1, bad + 1));
                }
            }
            let mut data = sheet.rows;
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
            if let Some(bad) = answer.values.iter().position(|value| !is_scalar(value)) {
                return Err(format!("the model answered value {} with a nested value", bad + 1));
            }
            let mut values = answer.values;
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
                assert_eq!(rows[0], vec![Value::String("A".into()), Value::from(1)]);
            }
            other => panic!("expected a sheet, got {other:?}"),
        }
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
        let answer = r#"{"columns":["A"],"rows":[[{"x":1}]]}"#;
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
        let answer = r#"{"values":["1 mg","2 mg","3 mg"]}"#;
        match interpret(&request, answer).expect("values") {
            Outcome::Values { values } => assert_eq!(values, vec![Value::String("1 mg".into()), Value::String("2 mg".into())]),
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
