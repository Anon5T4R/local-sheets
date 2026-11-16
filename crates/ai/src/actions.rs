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
    /// A free description -> a header and `rows` data rows, on a new sheet. `language` is the
    /// app's interface language name: the content and every text must be written in it.
    GenerateSheet { topic: String, rows: u32, language: String },
    /// The current column plus its neighbours -> `count` values down the column.
    FillColumn { header: String, context: String, count: u32 },
    /// A question about a cell plus the table around it -> one formula.
    Formula { question: String, cell: String, context: String },
    /// The headers and a sample of the selected table -> what each column holds, and whether the
    /// last row is a total. Nothing is generated: the app applies its own preset.
    ClassifyTable { range: String, header: String, sample: String },
}

/// A column's data kind: the closed vocabulary the model may label (never a format string).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Text,
    Currency,
    Percent,
    Date,
    Month,
}

impl Kind {
    /// The label, as the contract writes it.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Currency => "currency",
            Kind::Percent => "percent",
            Kind::Date => "date",
            Kind::Month => "month",
        }
    }

    /// The kind `name` labels, or [`Kind::Text`] for anything else (a closed vocabulary: an
    /// invented label never reaches a number format).
    pub fn from_id(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "currency" => Kind::Currency,
            "percent" | "percentage" => Kind::Percent,
            "date" => Kind::Date,
            "month" => Kind::Month,
            _ => Kind::Text,
        }
    }
}

/// One column of a generated sheet: its header and what the values are.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub title: String,
    pub kind: Kind,
}

/// The look the app applies after an answer. Its styles are the app's own: the model only labels
/// cells, and the preset never reaches the prompt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preset {
    /// No preset: the answer is written as it came.
    #[default]
    None,
    /// Header band, top row frozen, a number format per column, totals row set apart.
    Financial,
    /// A table: header band, autofit and banded rows.
    Table,
    /// Plain data: autofit only.
    Raw,
}

impl Preset {
    /// Every preset, in picker order (the default first).
    pub const ALL: [Preset; 4] = [Preset::None, Preset::Financial, Preset::Table, Preset::Raw];

    /// The id the dialog stores (stable across releases).
    pub fn id(self) -> &'static str {
        match self {
            Preset::None => "none",
            Preset::Financial => "financial",
            Preset::Table => "table",
            Preset::Raw => "raw",
        }
    }

    /// The name the picker shows.
    pub fn label(self) -> &'static str {
        match self {
            Preset::None => "None",
            Preset::Financial => "Financial",
            Preset::Table => "Simple table",
            Preset::Raw => "Raw data",
        }
    }

    /// The preset `id` names, or [`Preset::None`].
    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|preset| preset.id() == id).unwrap_or(Preset::None)
    }
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
    /// A new sheet: `columns` is the header row, `rows` the data, `total_row` a last row that
    /// aggregates the ones above it.
    Sheet { title: String, columns: Vec<Column>, rows: Vec<Vec<Cell>>, total_row: bool },
    /// Values for a column (top to bottom).
    Values { values: Vec<Cell> },
    /// One formula, starting with `=`.
    Formula { formula: String },
    /// What the selected table holds, column by column, and whether its last row is a total.
    Table { columns: Vec<Kind>, total_row: bool },
}

impl Request {
    /// The short label the status strip shows while the worker runs (translated by the window).
    pub fn label(&self) -> &'static str {
        match self {
            Request::GenerateSheet { .. } => "Generating a sheet",
            Request::FillColumn { .. } => "Filling the column",
            Request::Formula { .. } => "Writing the formula",
            Request::ClassifyTable { .. } => "Reading the table",
        }
    }

    /// The subject appended to the label (the topic, the column header, the cell, the range).
    pub fn subject(&self) -> &str {
        match self {
            Request::GenerateSheet { topic, .. } => topic,
            Request::FillColumn { header, .. } => header,
            Request::Formula { cell, .. } => cell,
            Request::ClassifyTable { range, .. } => range,
        }
    }
}

/// The conversation one request sends: a system turn and one user turn.
pub fn messages(request: &Request, lang: &str) -> Result<Vec<Msg>, String> {
    let prompts = for_lang(lang);
    let system = Msg::System(fill(prompts.system, &[("answer_in", prompts.answer_in)]));
    let user = match request {
        Request::GenerateSheet { topic, rows, language } => {
            let topic = topic.trim();
            if topic.is_empty() {
                return Err("the topic is empty".into());
            }
            fill(
                prompts.generate,
                &[
                    ("topic", topic),
                    ("rows", &rows.to_string()),
                    ("language", language),
                    ("answer_in", prompts.answer_in),
                    ("functions", ENGINE_FUNCTIONS),
                ],
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
        Request::ClassifyTable { range, header, sample } => {
            fill(prompts.classify, &[("range", range), ("header", header), ("sample", sample), ("answer_in", prompts.answer_in)])
        }
    };
    Ok(vec![system, Msg::User(user)])
}

/// Turns one answer into what the window applies. Structural problems are errors with the row that
/// failed; the requested count is a target, so a model that answers a few rows more or fewer is
/// accepted (the window applies what came).
pub fn interpret(request: &Request, answer: &str) -> Result<Outcome, String> {
    match request {
        Request::GenerateSheet { topic, rows, .. } => {
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
            let columns: Vec<Column> = sheet
                .columns
                .into_iter()
                .map(|column| match column {
                    ColumnAnswer::Title(title) => Column { title: title.trim().to_string(), kind: Kind::Text },
                    ColumnAnswer::Object { title, kind } => {
                        Column { title: title.trim().to_string(), kind: Kind::from_id(kind.as_deref().unwrap_or("text")) }
                    }
                })
                .collect();
            if let Some(index) = columns.iter().position(|column| column.title.is_empty()) {
                return Err(format!("the model answered column {} without a title", index + 1));
            }
            let total_row = sheet.last_row.as_deref().is_some_and(|role| role.eq_ignore_ascii_case("total"));
            Ok(Outcome::Sheet { title, columns, rows: data, total_row })
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
        Request::ClassifyTable { .. } => {
            let answer: ClassifyAnswer = extract_json(answer)?;
            if answer.columns.is_empty() {
                return Err("the model did not answer any column".into());
            }
            let columns = answer
                .columns
                .into_iter()
                .map(|column| match column {
                    ClassifiedColumn::Kind(kind) => Kind::from_id(&kind),
                    ClassifiedColumn::Object { kind } => Kind::from_id(kind.as_deref().unwrap_or("text")),
                })
                .collect();
            Ok(Outcome::Table { columns, total_row: answer.total_row })
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
    columns: Vec<ColumnAnswer>,
    #[serde(default)]
    rows: Vec<Vec<Value>>,
    /// `"total"` marks the last row as the totals row (the app draws it apart).
    #[serde(default)]
    last_row: Option<String>,
}

/// A column as the contract writes it (an object with a title and a kind) or as a model may still
/// answer it (the header alone, which is text).
#[derive(Deserialize)]
#[serde(untagged)]
enum ColumnAnswer {
    Title(String),
    Object {
        #[serde(default)]
        title: String,
        #[serde(default)]
        kind: Option<String>,
    },
}

#[derive(Deserialize)]
struct ClassifyAnswer {
    #[serde(default)]
    columns: Vec<ClassifiedColumn>,
    #[serde(default)]
    total_row: bool,
}

/// One classified column: a bare kind label or an object that carries it.
#[derive(Deserialize)]
#[serde(untagged)]
enum ClassifiedColumn {
    Kind(String),
    Object {
        #[serde(default)]
        kind: Option<String>,
    },
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
        Request::GenerateSheet { topic: "medicamentos".into(), rows: 3, language: "português".into() }
    }

    #[test]
    fn a_sheet_answer_becomes_columns_and_rows() {
        let answer = r#"{"title":"Medicamentos","columns":["Nome","Dose"],"rows":[["A",1],["B",2]]}"#;
        let outcome = interpret(&generate(), answer).expect("a sheet");
        match outcome {
            Outcome::Sheet { title, columns, rows, total_row } => {
                assert_eq!(title, "Medicamentos");
                let titles: Vec<&str> = columns.iter().map(|column| column.title.as_str()).collect();
                assert_eq!(titles, ["Nome", "Dose"]);
                assert!(columns.iter().all(|column| column.kind == Kind::Text), "string headers are text");
                assert!(!total_row);
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

    /// The preset vocabulary: each column says what it holds, and a last row that totals is marked.
    #[test]
    fn columns_carry_their_kind_and_a_total_row_is_marked() {
        let answer = r#"{"columns":[{"title":"Item","kind":"text"},{"title":"Cost","kind":"currency"},{"title":"Share","kind":"percent"}],"rows":[["Rent",1200,{"f":"=B2/B4"}]],"last_row":"total"}"#;
        match interpret(&generate(), answer).expect("a sheet") {
            Outcome::Sheet { columns, total_row, .. } => {
                assert_eq!(columns.iter().map(|column| column.kind).collect::<Vec<_>>(), [Kind::Text, Kind::Currency, Kind::Percent]);
                assert!(total_row, "the last row is the totals row");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A label outside the closed vocabulary is text: it never becomes a number format.
    #[test]
    fn an_invented_kind_is_text() {
        assert_eq!(Kind::from_id("money-ish"), Kind::Text);
        assert_eq!(Kind::from_id(""), Kind::Text);
        assert_eq!(Kind::from_id("PERCENT"), Kind::Percent);
        let answer = r#"{"columns":[{"title":"A","kind":"dinheiro"}],"rows":[[1]]}"#;
        match interpret(&generate(), answer).expect("a sheet") {
            Outcome::Sheet { columns, .. } => assert_eq!(columns[0].kind, Kind::Text),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_classified_table_answers_one_kind_per_column() {
        let request = Request::ClassifyTable { range: "A1:C4".into(), header: "A | B | C".into(), sample: "1 | 2 | 3".into() };
        let answer = r#"{"columns":["text","currency",{"kind":"percent"},"date"],"total_row":true}"#;
        match interpret(&request, answer).expect("roles") {
            Outcome::Table { columns, total_row } => {
                assert_eq!(columns, [Kind::Text, Kind::Currency, Kind::Percent, Kind::Date]);
                assert!(total_row);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn presets_round_trip_by_id_and_none_is_the_default() {
        assert_eq!(Preset::default(), Preset::None);
        assert_eq!(Preset::ALL.len(), 4);
        for preset in Preset::ALL {
            assert_eq!(Preset::from_id(preset.id()), preset);
            assert!(!preset.label().is_empty());
        }
        assert_eq!(Preset::from_id("unknown"), Preset::None);
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
        let request = Request::GenerateSheet { topic: "  ".into(), rows: 3, language: "en".into() };
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
