//! The AI surface: connections, the three sheet actions, the status strip and the dialogs.
//!
//! Everything the user configures is a [`local_ai::Connection`] (provider preset, base URL, model)
//! plus a key that lives in the OS keychain. The actions turn the sheet around the selection into a
//! [`Request`], the model streams an answer, and [`localsheets_ai::actions::interpret`] turns it into
//! an [`Outcome`] that is applied through the engine's own commands inside **one undo group** - so a
//! failed apply rolls back and the `.xlsx` is never written by the AI directly.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use egui::{Context, RichText, Ui};
use localsheets_ai::local_ai::{self, Connection};
use localsheets_ai::{AI_APP_ID, AI_APP_URL, Backend, Cell, Kind, LiveBackend, Outcome, Preset, Request};
use localsheets_engine::EngineError;
use localsheets_engine::Session;
use localsheets_engine::core::{CellRef, MAX_COLS, MAX_ROWS, RangeRef};
use serde_json::{Value, json};

use crate::SheetApp;

/// What a worker thread reports back.
enum Event {
    Delta(String),
    Done(Result<String, String>),
}

/// Where an outcome is applied, in engine terms.
#[derive(Clone)]
pub(crate) enum Apply {
    /// A new sheet (the outcome carries the cells), and the look the user picked for it.
    Sheet { preset: Preset },
    /// The target column: where it starts and the A1 range the values go into.
    Fill { start: CellRef, range: String },
    /// The cell the formula goes into.
    Formula { cell: CellRef },
    /// An existing table, formatted with the classifier's roles and the user's preset.
    Table { start: CellRef, end: CellRef, preset: Preset },
}

/// The last generate the user ran, so its dialog opens pre-filled (the suite's "repeat" gesture).
#[derive(Clone, Debug, PartialEq)]
pub struct LastGenerate {
    pub topic: String,
    pub rows: u32,
    pub preset: Preset,
}

/// The request in flight.
struct Running {
    request: Request,
    apply: Apply,
    rx: Receiver<Event>,
    /// `clock::monotonic_ms` when the request started (the strip's elapsed seconds).
    started_ms: f64,
    partial: String,
    cancelled: Arc<AtomicBool>,
}

/// A connection being edited (new or existing).
pub struct Draft {
    pub id: String,
    pub provider: String,
    pub label: String,
    pub base_url: String,
    pub model: String,
    pub key: String,
}

/// One of the action dialogs.
pub enum Dialog {
    Generate {
        topic: String,
        rows: u32,
        preset: Preset,
        error: Option<String>,
    },
    Fill {
        header: String,
        context: String,
        target: String,
        start: CellRef,
        count: u32,
        single: bool,
        error: Option<String>,
    },
    Formula {
        question: String,
        cell: CellRef,
        context: String,
        formula: Option<String>,
        error: Option<String>,
    },
    /// The classifier's dialog: what the selected table holds, and the look to give it.
    Format {
        preset: Preset,
        start: CellRef,
        end: CellRef,
        range: String,
        header: String,
        sample: String,
        error: Option<String>,
    },
}

/// The AI state the window owns.
pub struct Ai {
    pub backend: Arc<dyn Backend>,
    pub connections: Vec<Connection>,
    pub active: Option<String>,
    pub keys: HashMap<String, bool>,
    keys_loaded: bool,
    pub panel: bool,
    pub dialog: Option<Dialog>,
    pub draft: Option<Draft>,
    pub models: Option<Vec<String>>,
    fetching: Option<Receiver<Result<Vec<String>, String>>>,
    running: Option<Running>,
    pub error: Option<String>,
    /// The last generated sheet's request (repeat).
    pub last: Option<LastGenerate>,
}

impl Default for Ai {
    fn default() -> Self {
        let app = local_ai::App { id: AI_APP_ID, url: AI_APP_URL };
        let mut error = None;
        let connections = match Connection::load_all(&app) {
            Ok(connections) => connections,
            Err(e) => {
                error = Some(format!("the AI connections could not be read: {e}"));
                Vec::new()
            }
        };
        let active = connections.first().map(|connection| connection.id.clone());
        Self {
            backend: Arc::new(LiveBackend::new(AI_APP_ID, AI_APP_URL)),
            connections,
            active,
            keys: HashMap::new(),
            keys_loaded: false,
            panel: false,
            dialog: None,
            draft: None,
            models: None,
            fetching: None,
            running: None,
            error,
            last: None,
        }
    }
}

impl Ai {
    fn app() -> local_ai::App {
        local_ai::App { id: AI_APP_ID, url: AI_APP_URL }
    }

    fn active_connection(&self) -> Option<&Connection> {
        self.active.as_ref().and_then(|id| self.connections.iter().find(|connection| &connection.id == id))
    }

    /// Re-reads the keychain for every connection (the panel calls this once when it opens).
    fn refresh_keys(&mut self) {
        let app = Self::app();
        self.keys.clear();
        for connection in &self.connections {
            self.keys.insert(connection.id.clone(), connection.has_key(&app));
        }
        self.keys_loaded = true;
    }

    /// Starts one request on a worker thread. `lang` picks the prompt language.
    pub(crate) fn start(&mut self, ctx: &Context, request: Request, apply: Apply, lang: &str) -> Result<(), String> {
        let Some(connection) = self.active_connection().cloned() else {
            return Err("No AI connection yet. Open AI > Connections and add one.".into());
        };
        let messages = localsheets_ai::actions::messages(&request, lang)?;
        // Remember the request, so the dialog can come back pre-filled (repeat).
        if let (Request::GenerateSheet { topic, rows }, Apply::Sheet { preset }) = (&request, &apply) {
            self.last = Some(LastGenerate { topic: topic.clone(), rows: *rows, preset: *preset });
        }
        let (tx, rx) = mpsc::channel();
        let backend = self.backend.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let worker_ctx = ctx.clone();
        std::thread::Builder::new()
            .name("localsheets-ai".into())
            .spawn(move || {
                let mut on_delta = |delta: &str| {
                    if worker_cancelled.load(Ordering::Relaxed) {
                        return;
                    }
                    let _ = tx.send(Event::Delta(delta.to_string()));
                    worker_ctx.request_repaint();
                };
                let result = backend.stream(&connection, &messages, &mut on_delta);
                let _ = tx.send(Event::Done(result));
                worker_ctx.request_repaint();
            })
            .map_err(|e| format!("the AI worker could not start: {e}"))?;
        self.error = None;
        self.running = Some(Running { request, apply, rx, started_ms: crate::clock::monotonic_ms(), partial: String::new(), cancelled });
        Ok(())
    }

    /// Stops showing the answer. The request itself runs to its end in the worker (the shared
    /// client has no abort), so the result is discarded here instead.
    pub(crate) fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.cancelled.store(true, Ordering::Relaxed);
        }
    }

    /// Polls the worker and the model fetch; applies a finished outcome. Returns true when a
    /// command changed the workbook (the window then brings it into view).
    pub(crate) fn tick(&mut self, session: &mut Session) -> bool {
        self.tick_models();
        let Some(running) = self.running.as_mut() else { return false };
        let mut finished = None;
        loop {
            match running.rx.try_recv() {
                Ok(Event::Delta(delta)) => running.partial.push_str(&delta),
                Ok(Event::Done(result)) => {
                    finished = Some(result);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    finished = Some(Err("the AI worker stopped".into()));
                    break;
                }
            }
        }
        let Some(result) = finished else { return false };
        let running = match self.running.take() {
            Some(running) => running,
            None => return false,
        };
        if running.cancelled.load(Ordering::Relaxed) {
            return false;
        }
        let answer = match result {
            Ok(answer) => answer,
            Err(error) => {
                self.error = Some(error);
                return false;
            }
        };
        match localsheets_ai::actions::interpret(&running.request, &answer) {
            Ok(outcome) => match self.apply(session, &running.request, running.apply.clone(), outcome) {
                Ok(true) => true,
                Ok(false) => false,
                Err(error) => {
                    // The suite's gesture for a refused artifact action: the dialog comes back with
                    // the engine's reason and the user's parameters, ready to adjust.
                    self.reopen_after_refusal(&running.request, running.apply, error);
                    false
                }
            },
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    /// Reopens the dialog a refused action came from (Generate and Fill; the Formula action already
    /// answers in its own dialog). The reason goes into the dialog, not the strip.
    fn reopen_after_refusal(&mut self, request: &Request, apply: Apply, error: String) {
        self.dialog = match (request, apply) {
            (Request::GenerateSheet { topic, rows }, Apply::Sheet { preset }) => {
                Some(Dialog::Generate { topic: topic.clone(), rows: *rows, preset, error: Some(error) })
            }
            (Request::FillColumn { header, context, count }, Apply::Fill { start, range }) => Some(Dialog::Fill {
                header: header.clone(),
                context: context.clone(),
                target: range,
                start,
                count: *count,
                single: false,
                error: Some(error),
            }),
            _ => {
                self.error = Some(error);
                None
            }
        };
    }

    fn tick_models(&mut self) {
        let Some(rx) = &self.fetching else { return };
        match rx.try_recv() {
            Ok(Ok(models)) => {
                self.models = Some(models);
                self.fetching = None;
            }
            Ok(Err(error)) => {
                self.error = Some(error);
                self.fetching = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.fetching = None,
        }
    }

    /// Applies one outcome through the engine's commands. `Ok(true)` = the workbook changed,
    /// `Ok(false)` = nothing to do (the formula was refused and the dialog shows why), `Err` = the
    /// group rolled back and the message belongs in the strip.
    fn apply(&mut self, session: &mut Session, request: &Request, target: Apply, outcome: Outcome) -> Result<bool, String> {
        match (request, target, outcome) {
            (Request::GenerateSheet { .. }, Apply::Sheet { preset }, Outcome::Sheet { title, columns, rows, total_row }) => {
                let width = columns.len();
                let kinds: Vec<Kind> = columns.iter().map(|column| column.kind).collect();
                let data_rows = rows.len();
                session
                    .grouped("AI: generate sheet", |s| {
                        let inserted = match s.execute("home.insertSheet", json!({ "name": sheet_name(&title) })) {
                            Ok(value) => value,
                            // A name that clashes or is invalid falls back to the next free one.
                            Err(_) => s.execute("home.insertSheet", json!({}))?,
                        };
                        let index = inserted.get("index").and_then(Value::as_u64).unwrap_or(0);
                        // The new sheet is the one the formulas are entered in (and the one the
                        // engine evaluates them against).
                        s.execute("sheet.activate", json!({ "sheet": index }))?;
                        let mut matrix: Vec<Vec<Value>> = Vec::with_capacity(rows.len() + 1);
                        matrix.push(columns.into_iter().map(|column| Value::String(column.title)).collect());
                        let (data, formulas) = cells_matrix(rows, CellRef::new(1, 0));
                        matrix.extend(data);
                        let (matrix, post) = split_equals(matrix, CellRef::new(0, 0));
                        s.execute("range.setValues", json!({ "sheet": index, "range": "A1", "values": matrix, "parse": false }))?;
                        for (cell, text) in post {
                            s.execute("cell.set", json!({ "sheet": index, "cell": cell.a1(), "input": format!("'{text}") }))?;
                        }
                        write_formulas(s, &formulas)?;
                        apply_preset(
                            s,
                            preset,
                            &TableShape { start: CellRef::new(0, 0), width, rows: 1 + data_rows, kinds, total_row, header: true },
                        )?;
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
            (Request::ClassifyTable { .. }, Apply::Table { start, end, preset }, Outcome::Table { columns, total_row }) => {
                let width = end.col.saturating_sub(start.col) as usize + 1;
                let rows = end.row.saturating_sub(start.row) as usize + 1;
                session
                    .grouped("AI: format table", |s| {
                        apply_preset(s, preset, &TableShape { start, width, rows, kinds: columns, total_row, header: true })
                    })
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
            (Request::FillColumn { .. }, Apply::Fill { start, range }, Outcome::Values { values }) => {
                session
                    .grouped("AI: fill column", |s| {
                        let rows: Vec<Vec<Cell>> = values.into_iter().map(|cell| vec![cell]).collect();
                        let (matrix, formulas) = cells_matrix(rows, start);
                        let (matrix, post) = split_equals(matrix, start);
                        s.execute("range.setValues", json!({ "range": range, "values": matrix, "parse": false }))?;
                        for (cell, text) in post {
                            s.execute("cell.set", json!({ "cell": cell.a1(), "input": format!("'{text}") }))?;
                        }
                        write_formulas(s, &formulas)?;
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
            (Request::Formula { question, context, .. }, Apply::Formula { cell }, Outcome::Formula { formula }) => {
                // Validate without touching the sheet: a formula the engine cannot enter is shown
                // with the engine's reason in the dialog, not written.
                match session.execute("formulas.evaluate", json!({ "formula": formula, "cell": cell.a1() })) {
                    Ok(_) => {
                        session
                            .grouped("AI: formula", |s| {
                                s.execute("cell.set", json!({ "cell": cell.a1(), "input": formula }))?;
                                Ok(())
                            })
                            .map_err(|e| e.to_string())?;
                        Ok(true)
                    }
                    Err(error) => {
                        self.dialog = Some(Dialog::Formula {
                            question: question.clone(),
                            cell,
                            context: context.clone(),
                            formula: Some(formula),
                            error: Some(format!("{error}")),
                        });
                        Ok(false)
                    }
                }
            }
            _ => Err("the model answered a shape that does not fit this action".into()),
        }
    }

    /// Opens the generate dialog. It comes back pre-filled with the last request (the suite's
    /// "repeat" gesture), so asking again is one click.
    pub(crate) fn open_generate(&mut self) {
        let (topic, rows, preset) = self.last.as_ref().map_or((String::new(), 20, Preset::None), |last| (last.topic.clone(), last.rows, last.preset));
        self.dialog = Some(Dialog::Generate { topic, rows, preset, error: None });
    }

    /// Opens the classifier's dialog on the selected table: the model only labels the columns, the
    /// chosen preset is what formats them.
    pub(crate) fn open_format(&mut self, session: &Session) {
        let Some(context) = table_context(session) else {
            self.error = Some("Select the table to format first.".into());
            return;
        };
        self.dialog = Some(Dialog::Format {
            preset: Preset::Financial,
            start: context.start,
            end: context.end,
            range: context.range,
            header: context.header,
            sample: context.sample,
            error: None,
        });
    }

    /// Fills the current column. A multi-cell selection fixes the rows; a single cell asks how many
    /// rows (the dialog's count). The neighbouring columns are the context.
    pub(crate) fn open_fill(&mut self, session: &Session) {
        let Some(context) = selection_context(session) else {
            self.error = Some("Open a workbook first.".into());
            return;
        };
        let (start, count, single) = if context.range.is_single() {
            (context.start, 20, true)
        } else {
            let rows = context.range.end.row.saturating_sub(context.start.row) + 1;
            (CellRef::new(context.range.start.row, context.start.col), rows.min(500), false)
        };
        let target = range_a1(start, count);
        self.dialog = Some(Dialog::Fill { header: context.header, context: context.text, target, start, count, single, error: None });
    }

    pub(crate) fn open_formula(&mut self, session: &Session) {
        let Some(context) = selection_context(session) else {
            self.error = Some("Open a workbook first.".into());
            return;
        };
        self.dialog = Some(Dialog::Formula { question: String::new(), cell: context.start, context: context.text, formula: None, error: None });
    }
}

/// What a cell selection gives the prompts: where the target starts, the selected range, the header
/// row and a capped text table around it.
struct SelectionContext {
    start: CellRef,
    range: RangeRef,
    header: String,
    text: String,
}

fn selection_context(session: &Session) -> Option<SelectionContext> {
    let doc = session.active()?;
    let range = doc.selection.current();
    let start = doc.selection.active;
    let sheet = doc.wb.sheet(doc.wb.active_sheet())?;
    let region = current_region(sheet, start);
    let header_row = if start.row > 0 { start.row - 1 } else { region.start.row };
    let header = (region.start.col..=region.end.col)
        .map(|col| sheet.input_text(CellRef::new(header_row, col)))
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    let mut text = String::new();
    for row in region.start.row..=region.end.row.min(region.start.row + 7) {
        let mut line = String::new();
        for col in region.start.col..=region.end.col.min(region.start.col + 5) {
            if col > region.start.col {
                line.push_str(" | ");
            }
            line.push_str(&sheet.input_text(CellRef::new(row, col)));
        }
        text.push_str(&line);
        text.push('\n');
    }
    Some(SelectionContext { start, range, header, text })
}

/// The current region around `at`: the contiguous block of non-empty cells (the table the cell
/// belongs to), bounded by a scan, not the whole sheet.
fn current_region(sheet: &localsheets_engine::model::Sheet, at: CellRef) -> RangeRef {
    let empty = |row: u32, col: u32| sheet.input_text(CellRef::new(row, col)).trim().is_empty();
    let mut left = at.col;
    while left > 0 && !empty(at.row, left - 1) {
        left -= 1;
    }
    let mut right = at.col;
    while right + 1 < MAX_COLS && !empty(at.row, right + 1) {
        right += 1;
    }
    let mut top = at.row;
    while top > 0 && (left..=right).any(|col| !empty(top - 1, col)) {
        top -= 1;
    }
    let mut bottom = at.row;
    while bottom + 1 < MAX_ROWS && (left..=right).any(|col| !empty(bottom + 1, col)) {
        bottom += 1;
    }
    RangeRef { start: CellRef::new(top, left), end: CellRef::new(bottom, right) }
}

/// The A1 range of `count` rows from `start`, one column.
fn range_a1(start: CellRef, count: u32) -> String {
    let end = start.offset(count.saturating_sub(1) as i64, 0).unwrap_or(start);
    format!("{}:{}", start.a1(), end.a1())
}

/// A sheet name the workbook accepts: no `[]:*?/\`, no control characters, at most 31.
fn sheet_name(title: &str) -> String {
    let name: String = title.chars().filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\') && !c.is_control()).take(31).collect();
    let name = name.trim();
    if name.is_empty() { "AI".to_string() } else { name.to_string() }
}

/// The `range.setValues` matrix and the formula cells of an answer. A formula cell is left empty in
/// the matrix (the value pass never writes a formula as text) and is entered by [`write_formulas`]
/// once the cells it references exist.
fn cells_matrix(rows: Vec<Vec<Cell>>, start: CellRef) -> (Vec<Vec<Value>>, Vec<(CellRef, String)>) {
    let mut matrix = Vec::with_capacity(rows.len());
    let mut formulas = Vec::new();
    for (row, cells) in rows.into_iter().enumerate() {
        let mut line = Vec::with_capacity(cells.len());
        for (col, cell) in cells.into_iter().enumerate() {
            match cell {
                Cell::Value(value) => line.push(value),
                Cell::Formula(formula) => {
                    line.push(Value::Null);
                    if let Some(at) = start.offset(row as i64, col as i64) {
                        formulas.push((at, formula));
                    }
                }
            }
        }
        matrix.push(line);
    }
    (matrix, formulas)
}

/// Enters the answer's formulas through `cell.set`, **after** the engine has evaluated each one. A
/// formula it cannot enter, or one that names a function the sheet doesn't have, stops the whole
/// action: no broken formula is ever written.
fn write_formulas(s: &mut Session, formulas: &[(CellRef, String)]) -> Result<(), EngineError> {
    for (cell, formula) in formulas {
        let value = s.execute("formulas.evaluate", json!({ "formula": formula, "cell": cell.a1() }))?;
        if value.get("error").and_then(Value::as_str) == Some("#NAME?") {
            return Err(EngineError::Other(format!("the formula in {} uses a function or name the spreadsheet doesn't have", cell.a1())));
        }
        s.execute("cell.set", json!({ "cell": cell.a1(), "input": formula }))?;
    }
    Ok(())
}

/// The cells a preset formats: where the table starts, how wide and tall it is, and what the
/// classifier (or the generator) said about each column.
struct TableShape {
    start: CellRef,
    width: usize,
    rows: usize,
    kinds: Vec<Kind>,
    total_row: bool,
    header: bool,
}

impl TableShape {
    /// The A1 range of the whole table.
    fn range(&self) -> String {
        let end = self.start.offset(self.rows.saturating_sub(1) as i64, self.width.saturating_sub(1) as i64).unwrap_or(self.start);
        RangeRef { start: self.start, end }.a1()
    }

    /// The A1 range of the header row.
    fn header_range(&self) -> String {
        let end = self.start.offset(0, self.width.saturating_sub(1) as i64).unwrap_or(self.start);
        RangeRef { start: self.start, end }.a1()
    }

    /// The A1 range of the last row (the totals row of a financial table).
    fn last_row_range(&self) -> String {
        let Some(start) = self.start.offset(self.rows.saturating_sub(1) as i64, 0) else { return self.start.a1() };
        let end = start.offset(0, self.width.saturating_sub(1) as i64).unwrap_or(start);
        RangeRef { start, end }.a1()
    }

    /// The A1 range of one data column, below the header.
    fn column_range(&self, index: usize) -> Option<String> {
        let first = if self.header { 1 } else { 0 };
        let last = self.rows.checked_sub(1)?;
        if last < first {
            return None;
        }
        let start = self.start.offset(first as i64, index as i64)?;
        let end = self.start.offset(last as i64, index as i64)?;
        Some(RangeRef { start, end }.a1())
    }
}

/// The header band's fill (a light blue that reads on both themes' grids).
const HEADER_FILL: &str = "#DDEBF7";

/// Applies the look the user picked. Style is the app's: the answer only says what each column
/// holds (a closed vocabulary) and whether the last row totals.
fn apply_preset(s: &mut Session, preset: Preset, shape: &TableShape) -> Result<(), EngineError> {
    if preset == Preset::None {
        return Ok(());
    }
    if preset != Preset::Raw && shape.header {
        s.execute("home.formatCells", json!({ "range": shape.header_range(), "style": { "font": { "bold": true } } }))?;
        s.execute("home.fillColor", json!({ "range": shape.header_range(), "color": HEADER_FILL }))?;
    }
    if preset == Preset::Financial {
        s.execute("view.freezeTopRow", json!({}))?;
    }
    if preset != Preset::Raw {
        for (index, kind) in shape.kinds.iter().enumerate().take(shape.width) {
            let (Some(mut params), Some(range)) = (number_format(*kind), shape.column_range(index)) else { continue };
            params["range"] = json!(range);
            s.execute("home.numberFormat", params)?;
        }
    }
    if shape.total_row && preset != Preset::Raw {
        s.execute("home.borders", json!({ "range": shape.last_row_range(), "preset": "top", "style": "thin" }))?;
    }
    if preset == Preset::Table {
        s.execute("home.formatAsTable", json!({ "range": shape.range(), "style": "TableStyleMedium2", "header": shape.header }))?;
    }
    s.execute("home.autofitColumnWidth", json!({}))?;
    Ok(())
}

/// The number format a kind asks for. The kind is the model's label; the format is ours.
fn number_format(kind: Kind) -> Option<Value> {
    match kind {
        Kind::Text => None,
        Kind::Currency => Some(json!({ "format": "Currency" })),
        Kind::Percent => Some(json!({ "format": "Percentage" })),
        Kind::Date => Some(json!({ "format": "Short Date" })),
        Kind::Month => Some(json!({ "code": "mmm yyyy" })),
    }
}

/// The selected table as the classifier needs it: its range, the header row and a text sample.
struct TableContext {
    start: CellRef,
    end: CellRef,
    range: String,
    header: String,
    sample: String,
}

fn table_context(session: &Session) -> Option<TableContext> {
    let doc = session.active()?;
    let range = doc.selection.current();
    let sheet = doc.wb.sheet(doc.wb.active_sheet())?;
    let width = range.end.col.saturating_sub(range.start.col) as usize + 1;
    let rows = range.end.row.saturating_sub(range.start.row) as usize + 1;
    let row_text = |row: u32| (0..width).map(|col| sheet.input_text(CellRef::new(row, range.start.col + col as u32))).collect::<Vec<_>>().join(" | ");
    let header = row_text(range.start.row);
    let mut sample = String::new();
    for offset in 1..rows.min(9) {
        sample.push_str(&row_text(range.start.row + offset as u32));
        sample.push('\n');
    }
    Some(TableContext { start: range.start, end: range.end, range: range.a1(), header, sample })
}

/// The data cells as `range.setValues` takes them, with text that starts with `=` moved out: the
/// command always reads a leading `=` as a formula, so those cells are written by `cell.set` with a
/// leading `'` (typed input strips it and forces text). Nothing the model wrote becomes a formula by
/// accident.
fn split_equals(matrix: Vec<Vec<Value>>, start: CellRef) -> (Vec<Vec<Value>>, Vec<(CellRef, String)>) {
    let mut out = Vec::with_capacity(matrix.len());
    let mut post = Vec::new();
    for (row, values) in matrix.into_iter().enumerate() {
        let mut line = Vec::with_capacity(values.len());
        for (col, value) in values.into_iter().enumerate() {
            if let Value::String(text) = &value
                && text.starts_with('=')
                && let Some(cell) = start.offset(row as i64, col as i64)
            {
                line.push(Value::Null);
                post.push((cell, text.clone()));
            } else {
                line.push(value);
            }
        }
        out.push(line);
    }
    (out, post)
}

/// The error banner and the in-flight status, drawn **before** the grid (the grid takes the rest of
/// the window, so a strip after it would be clipped).
pub(crate) fn status_strip(app: &mut SheetApp, ui: &mut Ui) {
    let mut dismiss = false;
    if let Some(error) = app.ai.error.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(ui.visuals().error_fg_color, error);
            if ui.button(app.ui.language.tr("Dismiss")).clicked() {
                dismiss = true;
            }
        });
    }
    if dismiss {
        app.ai.error = None;
    }
    let Some(running) = app.ai.running.as_ref() else { return };
    let label = app.ui.language.tr(running.request.label()).to_string();
    let subject = running.request.subject().to_string();
    let elapsed = ((crate::clock::monotonic_ms() - running.started_ms) / 1000.0).max(0.0) as u64;
    let tail: String = running.partial.chars().rev().take(80).collect::<String>().chars().rev().collect::<String>();
    let mut stop = false;
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(format!("{label}: {subject}"));
        ui.label(RichText::new(format!("{elapsed}s")).weak());
        if !tail.trim().is_empty() {
            ui.label(RichText::new(tail.replace('\n', " ")).weak());
        }
        if ui.button(app.ui.language.tr("Stop")).clicked() {
            stop = true;
        }
    });
    if stop {
        app.ai.stop();
    }
}

/// The connections window and the action dialogs, drawn last so they sit above the grid.
pub(crate) fn windows(app: &mut SheetApp, ctx: &Context) {
    connections_window(app, ctx);
    dialog_window(app, ctx);
}

fn connections_window(app: &mut SheetApp, ctx: &Context) {
    if !app.ai.panel {
        return;
    }
    if !app.ai.keys_loaded {
        app.ai.refresh_keys();
    }
    let mut open = true;
    let mut add = false;
    let mut edit: Option<Connection> = None;
    let mut delete: Option<String> = None;
    let mut activate: Option<String> = None;
    let mut save = false;
    let mut fetch = false;
    let mut close_draft = false;
    let language = app.ui.language;
    egui::Window::new(language.tr("AI connections"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(540.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(RichText::new(language.tr("The key lives in the OS keychain; the connection file never holds it.")).weak());
            ui.add_space(4.0);
            if app.ai.connections.is_empty() {
                ui.label(language.tr("No connections yet."));
            }
            for connection in &app.ai.connections {
                ui.horizontal(|ui| {
                    let active = app.ai.active.as_deref() == Some(connection.id.as_str());
                    if ui.selectable_label(active, format!("{} · {} · {}", connection.label, connection.provider, connection.model)).clicked() {
                        activate = Some(connection.id.clone());
                    }
                    if app.ai.keys.get(&connection.id).copied().unwrap_or(false) {
                        ui.label(RichText::new(language.tr("key saved")).weak());
                    }
                    if ui.small_button(language.tr("Edit")).clicked() {
                        edit = Some(connection.clone());
                    }
                    if ui.small_button(language.tr("Delete")).clicked() {
                        delete = Some(connection.id.clone());
                    }
                });
            }
            ui.add_space(6.0);
            if ui.button(language.tr("Add connection")).clicked() {
                add = true;
            }
            if let Some(draft) = app.ai.draft.as_mut() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(language.tr("Provider"));
                    craft_ui::ComboBox::from_id_salt("ai-provider").selected_text(draft.provider.clone()).show_ui(ui, |ui| {
                        for provider in local_ai::PROVIDERS {
                            if ui.selectable_label(draft.provider == provider.id, provider.label).clicked() {
                                draft.provider = provider.id.to_string();
                                if !provider.base_url.is_empty() {
                                    draft.base_url = provider.base_url.to_string();
                                }
                                if draft.label.is_empty() {
                                    draft.label = provider.label.to_string();
                                }
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(language.tr("Label"));
                    ui.text_edit_singleline(&mut draft.label);
                });
                ui.horizontal(|ui| {
                    ui.label(language.tr("Base URL"));
                    ui.text_edit_singleline(&mut draft.base_url);
                });
                ui.horizontal(|ui| {
                    ui.label(language.tr("Model"));
                    if let Some(models) = &app.ai.models {
                        craft_ui::ComboBox::from_id_salt("ai-model").selected_text(draft.model.clone()).show_ui(ui, |ui| {
                            for model in models {
                                if ui.selectable_label(draft.model == *model, model).clicked() {
                                    draft.model = model.clone();
                                }
                            }
                        });
                    } else {
                        ui.text_edit_singleline(&mut draft.model);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Fetch models")).clicked() {
                        fetch = true;
                    }
                    if app.ai.fetching.is_some() {
                        ui.spinner();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(language.tr("API key"));
                    ui.add(egui::TextEdit::singleline(&mut draft.key).password(true));
                });
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Save connection")).clicked() {
                        save = true;
                    }
                    if ui.button(language.tr("Cancel")).clicked() {
                        close_draft = true;
                    }
                });
            }
        });
    app.ai.panel = open;
    let app_id = Ai::app();
    if add {
        let provider = local_ai::PROVIDERS.iter().find(|provider| provider.id == "openai").or_else(|| local_ai::PROVIDERS.first());
        let (id, base, label) = provider.map(|provider| (provider.id, provider.base_url, provider.label)).unwrap_or(("custom", "", "Custom"));
        app.ai.draft = Some(Draft {
            id: format!("conn-{}", crate::now_ms() as u64),
            provider: id.to_string(),
            label: label.to_string(),
            base_url: base.to_string(),
            model: String::new(),
            key: String::new(),
        });
        app.ai.models = None;
    }
    if let Some(connection) = edit {
        app.ai.draft = Some(Draft {
            id: connection.id.clone(),
            provider: connection.provider.clone(),
            label: connection.label.clone(),
            base_url: connection.base_url.clone(),
            model: connection.model.clone(),
            key: String::new(),
        });
        app.ai.models = None;
    }
    if let Some(id) = delete
        && let Some(connection) = app.ai.connections.iter().find(|connection| connection.id == id).cloned()
    {
        let _ = connection.clear_key(&app_id);
        app.ai.connections.retain(|connection| connection.id != id);
        let _ = Connection::save_all(&app_id, &app.ai.connections);
        app.ai.keys.remove(&id);
        if app.ai.active.as_deref() == Some(id.as_str()) {
            app.ai.active = app.ai.connections.first().map(|connection| connection.id.clone());
        }
    }
    if let Some(id) = activate {
        app.ai.active = Some(id);
    }
    if fetch && let Some(draft) = &app.ai.draft {
        let connection = Connection {
            id: draft.id.clone(),
            provider: draft.provider.clone(),
            label: draft.label.clone(),
            base_url: draft.base_url.clone(),
            model: draft.model.clone(),
        };
        let key = if draft.key.is_empty() { connection.key(&app_id).unwrap_or_default() } else { draft.key.clone() };
        let (tx, rx) = mpsc::channel();
        let worker = ctx.clone();
        std::thread::spawn(move || {
            let app = Ai::app();
            let result = local_ai::list_models_with_key(&app, &connection, &key);
            let _ = tx.send(result);
            worker.request_repaint();
        });
        app.ai.fetching = Some(rx);
    }
    if close_draft {
        app.ai.draft = None;
        app.ai.models = None;
    }
    if save && let Some(draft) = app.ai.draft.take() {
        let connection = Connection {
            id: draft.id.clone(),
            provider: draft.provider.clone(),
            label: draft.label.clone(),
            base_url: draft.base_url.clone(),
            model: draft.model.clone(),
        };
        if !draft.key.is_empty()
            && let Err(error) = connection.set_key(&app_id, &draft.key)
        {
            app.ai.error = Some(format!("the key could not be saved: {error}"));
        }
        if let Some(existing) = app.ai.connections.iter_mut().find(|connection| connection.id == draft.id) {
            *existing = connection;
        } else {
            app.ai.connections.push(connection);
        }
        if let Err(error) = Connection::save_all(&app_id, &app.ai.connections) {
            app.ai.error = Some(format!("the AI connections could not be saved: {error}"));
        }
        app.ai.active = app.ai.connections.first().map(|connection| connection.id.clone());
        app.ai.keys_loaded = false;
        app.ai.models = None;
    }
}

fn dialog_window(app: &mut SheetApp, ctx: &Context) {
    let Some(mut dialog) = app.ai.dialog.take() else { return };
    let language = app.ui.language;
    let mut run: Option<(Request, Apply)> = None;
    let mut close = false;
    let mut error: Option<String> = None;
    let title = match &dialog {
        Dialog::Generate { .. } => language.tr("Generate sheet"),
        Dialog::Fill { .. } => language.tr("Fill column"),
        Dialog::Formula { .. } => language.tr("Formula"),
        Dialog::Format { .. } => language.tr("Format table"),
    };
    egui::Window::new(title).collapsible(false).resizable(false).default_width(460.0).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(
        ctx,
        |ui| match &mut dialog {
            Dialog::Generate { topic, rows, preset, error: dialog_error } => {
                ui.label(language.tr("Describe the sheet: a topic, or a whole text to organize."));
                ui.add(egui::TextEdit::multiline(topic).desired_rows(4).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.label(language.tr("Rows"));
                    ui.add(egui::DragValue::new(rows).range(1..=200));
                });
                preset_picker(ui, preset, language);
                if let Some(message) = dialog_error.as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Generate")).clicked() {
                        run = Some((Request::GenerateSheet { topic: topic.clone(), rows: *rows }, Apply::Sheet { preset: *preset }));
                    }
                    if ui.button(language.tr("Cancel")).clicked() {
                        close = true;
                    }
                });
            }
            Dialog::Format { preset, start, end, range, header, sample, error: dialog_error } => {
                ui.label(language.tr("The model reads the table; the preset is what formats it. No data is written."));
                ui.horizontal(|ui| {
                    ui.label(language.tr("Range"));
                    ui.label(RichText::new(range.clone()).monospace());
                });
                ui.label(RichText::new(header.clone()).monospace().weak());
                ui.label(RichText::new(sample.trim_end().to_string()).monospace().weak().small());
                preset_picker(ui, preset, language);
                if let Some(message) = dialog_error.as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Apply")).clicked() {
                        run = Some((
                            Request::ClassifyTable { range: range.clone(), header: header.clone(), sample: sample.clone() },
                            Apply::Table { start: *start, end: *end, preset: *preset },
                        ));
                    }
                    if ui.button(language.tr("Cancel")).clicked() {
                        close = true;
                    }
                });
            }
            Dialog::Fill { header, context, target, start, count, single, error: dialog_error } => {
                ui.label(language.tr("Fill the current column from the neighbouring columns."));
                ui.horizontal(|ui| {
                    ui.label(language.tr("Target"));
                    ui.label(RichText::new(target.clone()).monospace());
                });
                if !header.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(language.tr("Header"));
                        ui.label(RichText::new(header.clone()).monospace());
                    });
                }
                if *single {
                    ui.horizontal(|ui| {
                        ui.label(language.tr("Rows"));
                        ui.add(egui::DragValue::new(count).range(1..=500));
                    });
                }
                ui.label(RichText::new(context.clone()).monospace().weak().small());
                if let Some(message) = dialog_error.as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Fill")).clicked() {
                        let range = range_a1(*start, *count);
                        run = Some((
                            Request::FillColumn { header: header.clone(), context: context.clone(), count: *count },
                            Apply::Fill { start: *start, range },
                        ));
                    }
                    if ui.button(language.tr("Cancel")).clicked() {
                        close = true;
                    }
                });
            }
            Dialog::Formula { question, cell, context, formula, error: dialog_error } => {
                ui.horizontal(|ui| {
                    ui.label(language.tr("Cell"));
                    ui.label(RichText::new(cell.a1()).monospace());
                });
                ui.label(language.tr("Ask for the formula in plain language."));
                ui.add(egui::TextEdit::multiline(question).desired_rows(2).desired_width(f32::INFINITY));
                ui.label(RichText::new(context.clone()).monospace().weak().small());
                if let Some(formula) = formula.as_ref() {
                    ui.label(RichText::new(formula.clone()).monospace());
                }
                if let Some(message) = dialog_error.as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Write formula")).clicked() {
                        run = Some((
                            Request::Formula { question: question.clone(), cell: cell.a1(), context: context.clone() },
                            Apply::Formula { cell: *cell },
                        ));
                    }
                    if ui.button(language.tr("Cancel")).clicked() {
                        close = true;
                    }
                });
            }
        },
    );
    if let Some((request, apply)) = run {
        let lang = app.ui.language.code();
        match app.ai.start(ctx, request, apply, lang) {
            Ok(()) => close = true,
            Err(message) => error = Some(message),
        }
    }
    if close {
        return;
    }
    set_dialog_error(&mut dialog, error);
    app.ai.dialog = Some(dialog);
}

fn set_dialog_error(dialog: &mut Dialog, error: Option<String>) {
    if let Some(message) = error {
        match dialog {
            Dialog::Generate { error: dialog_error, .. }
            | Dialog::Fill { error: dialog_error, .. }
            | Dialog::Formula { error: dialog_error, .. }
            | Dialog::Format { error: dialog_error, .. } => {
                *dialog_error = Some(message);
            }
        }
    }
}

/// The preset row every artifact dialog has: "None" first (the default), so a look is always
/// opt-in. The labels are ours; the model never sees the preset.
fn preset_picker(ui: &mut Ui, preset: &mut Preset, language: crate::i18n::Language) {
    ui.horizontal(|ui| {
        ui.label(language.tr("Preset"));
        for option in Preset::ALL {
            if ui.selectable_label(*preset == option, language.tr(option.label())).clicked() {
                *preset = option;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use localsheets_ai::Column;
    use localsheets_ai::local_ai::{self as la, Msg};

    fn no_backend() -> Arc<dyn Backend> {
        Arc::new(LiveBackend::new(AI_APP_ID, AI_APP_URL))
    }

    fn ai() -> Ai {
        Ai {
            backend: no_backend(),
            connections: Vec::new(),
            active: None,
            keys: HashMap::new(),
            keys_loaded: true,
            panel: false,
            dialog: None,
            draft: None,
            models: None,
            fetching: None,
            running: None,
            error: None,
            last: None,
        }
    }

    fn session() -> Session {
        let mut session = Session::new();
        session.new_workbook();
        session
    }

    fn value(session: &mut Session, at: &str) -> Value {
        session.execute("cell.get", json!({ "cell": at })).unwrap()["value"].clone()
    }

    fn undo_steps(session: &Session) -> usize {
        session.active().map_or(0, |doc| doc.undo.len())
    }

    #[test]
    fn a_generated_sheet_lands_in_a_new_sheet_in_one_undo_step() {
        let mut session = session();
        let before = undo_steps(&session);
        let mut ai = ai();
        let request = Request::GenerateSheet { topic: "Budget".into(), rows: 2 };
        let outcome = Outcome::Sheet {
            title: "Budget".into(),
            columns: vec![Column { title: "Item".into(), kind: Kind::Text }, Column { title: "Cost".into(), kind: Kind::Currency }],
            rows: vec![vec![Cell::Value(json!("Rent")), Cell::Value(json!(1200.0))], vec![Cell::Value(json!("Food")), Cell::Value(json!(400.0))]],
            total_row: false,
        };
        assert_eq!(ai.apply(&mut session, &request, Apply::Sheet { preset: Preset::None }, outcome), Ok(true));
        assert_eq!(undo_steps(&session), before + 1, "the whole action is one undo step");
        assert_eq!(value(&mut session, "A1"), json!("Item"));
        assert_eq!(value(&mut session, "B1"), json!("Cost"));
        assert_eq!(value(&mut session, "A3"), json!("Food"));
        assert_eq!(value(&mut session, "B3"), json!(400.0));
        session.execute("edit.undo", json!({})).unwrap();
        assert!(session.active().is_some_and(|doc| doc.wb.active_sheet() == 0), "one undo takes the whole sheet back");
    }

    #[test]
    fn a_filled_value_that_starts_with_equals_stays_text() {
        let mut session = session();
        let mut ai = ai();
        let request = Request::FillColumn { header: "Note".into(), context: String::new(), count: 2 };
        let outcome = Outcome::Values { values: vec![Cell::Value(json!("=SUM(A1:A2)")), Cell::Value(json!("plain"))] };
        let start = CellRef::new(0, 0);
        let range = range_a1(start, 2);
        assert_eq!(ai.apply(&mut session, &request, Apply::Fill { start, range }, outcome), Ok(true));
        // The command would read a leading `=` as a formula: the text is written as text instead.
        assert_eq!(value(&mut session, "A1"), json!("=SUM(A1:A2)"));
        assert_eq!(value(&mut session, "A2"), json!("plain"));
    }

    /// The bug of 2026-11-18: a column the model was asked to compute came back as invented numbers.
    /// Now a formula cell is entered through the engine and the column shows the computed values.
    #[test]
    fn a_formula_column_is_written_and_computed() {
        let mut session = session();
        let mut ai = ai();
        session.execute("cell.set", json!({ "cell": "A1", "input": "2" })).unwrap();
        session.execute("cell.set", json!({ "cell": "B1", "input": "3" })).unwrap();
        let request = Request::FillColumn { header: "Sum".into(), context: String::new(), count: 1 };
        let outcome = Outcome::Values { values: vec![Cell::Formula("=A1+B1".into())] };
        let start = CellRef::new(0, 2);
        let range = range_a1(start, 1);
        assert_eq!(ai.apply(&mut session, &request, Apply::Fill { start, range }, outcome), Ok(true));
        let got = session.execute("cell.get", json!({ "cell": "C1" })).unwrap();
        assert_eq!(got["formula"], json!("=A1+B1"), "the cell holds the formula, not a number");
        assert_eq!(got["value"], json!(5.0), "and the engine computed it");
    }

    #[test]
    fn a_formula_the_engine_cannot_enter_writes_nothing() {
        let mut session = session();
        let mut ai = ai();
        let request = Request::FillColumn { header: "Sum".into(), context: String::new(), count: 1 };
        let outcome = Outcome::Values { values: vec![Cell::Formula("=SUM(1;2)".into())] };
        let start = CellRef::new(0, 0);
        let range = range_a1(start, 1);
        let error = ai.apply(&mut session, &request, Apply::Fill { start, range }, outcome).expect_err("refused");
        assert!(error.contains("problem with this formula"), "{error}");
        assert_eq!(value(&mut session, "A1"), Value::Null, "the sheet is untouched");
    }

    /// A name the sheet doesn't know is the other way a model invents a function.
    #[test]
    fn a_formula_with_an_unknown_function_writes_nothing() {
        let mut session = session();
        let mut ai = ai();
        let request = Request::FillColumn { header: "Sum".into(), context: String::new(), count: 1 };
        let outcome = Outcome::Values { values: vec![Cell::Formula("=INVENTED(A1)".into())] };
        let start = CellRef::new(0, 0);
        let range = range_a1(start, 1);
        let error = ai.apply(&mut session, &request, Apply::Fill { start, range }, outcome).expect_err("refused");
        assert!(error.contains("doesn't have"), "{error}");
        assert_eq!(value(&mut session, "A1"), Value::Null, "the sheet is untouched");
    }

    /// The preset is the app's own look: the answer only labels the columns. Header band, a number
    /// format per kind and the totals row set apart.
    #[test]
    fn a_financial_preset_formats_the_answer() {
        let mut session = session();
        let mut ai = ai();
        let request = Request::GenerateSheet { topic: "Budget".into(), rows: 2 };
        let outcome = Outcome::Sheet {
            title: "Budget".into(),
            columns: vec![Column { title: "Item".into(), kind: Kind::Text }, Column { title: "Cost".into(), kind: Kind::Currency }],
            rows: vec![
                vec![Cell::Value(json!("Rent")), Cell::Value(json!(1200.0))],
                vec![Cell::Value(json!("Total")), Cell::Formula("=SUM(B2:B2)".into())],
            ],
            total_row: true,
        };
        assert_eq!(ai.apply(&mut session, &request, Apply::Sheet { preset: Preset::Financial }, outcome), Ok(true));
        let a1 = session.execute("cell.get", json!({ "cell": "A1" })).unwrap();
        assert_eq!(a1["style"]["font"]["bold"], json!(true), "header band: {}", a1["style"]);
        let b2 = session.execute("cell.get", json!({ "cell": "B2" })).unwrap();
        assert_ne!(b2["style"]["num_fmt"], json!("General"), "currency column: {}", b2["style"]);
        let a3 = session.execute("cell.get", json!({ "cell": "A3" })).unwrap();
        assert!(!a3["style"]["borders"]["top"].is_null(), "totals row set apart: {}", a3["style"]);
    }

    /// The classifier writes no data: it labels what is selected and the preset formats it.
    #[test]
    fn the_classifier_formats_an_existing_table_without_writing_data() {
        let mut session = session();
        session
            .execute("range.setValues", json!({ "range": "A1:C3", "values": [["Item", "Cost", "Share"], ["Rent", 1200, 0.5], ["Total", 1200, 0.5]] }))
            .unwrap();
        let mut ai = ai();
        let request = Request::ClassifyTable { range: "A1:C3".into(), header: String::new(), sample: String::new() };
        let outcome = Outcome::Table { columns: vec![Kind::Text, Kind::Currency, Kind::Percent], total_row: true };
        let apply = Apply::Table { start: CellRef::new(0, 0), end: CellRef::new(2, 2), preset: Preset::Financial };
        assert_eq!(ai.apply(&mut session, &request, apply, outcome), Ok(true));
        assert_eq!(value(&mut session, "B2"), json!(1200.0), "no value is rewritten");
        assert_eq!(value(&mut session, "C2"), json!(0.5));
        for cell in ["B2", "C2"] {
            let got = session.execute("cell.get", json!({ "cell": cell })).unwrap();
            assert_ne!(got["style"]["num_fmt"], json!("General"), "{cell} carries its column's format: {}", got["style"]);
        }
        let a3 = session.execute("cell.get", json!({ "cell": "A3" })).unwrap();
        assert!(!a3["style"]["borders"]["top"].is_null(), "totals row set apart: {}", a3["style"]);
    }

    /// The suite's gesture: an engine refusal comes back in the dialog, with the user's own
    /// parameters, so a retry is one edit away.
    #[test]
    fn a_refused_fill_reopens_its_dialog_with_the_reason() {
        let mut ai = ai();
        let request = Request::FillColumn { header: "Sum".into(), context: "ctx".into(), count: 3 };
        let start = CellRef::new(1, 2);
        let range = range_a1(start, 3);
        ai.reopen_after_refusal(&request, Apply::Fill { start, range: range.clone() }, "the reason".into());
        match ai.dialog {
            Some(Dialog::Fill { header, context, target, start: at, count, error, .. }) => {
                assert_eq!(header, "Sum");
                assert_eq!(context, "ctx");
                assert_eq!(target, range);
                assert_eq!(at, start);
                assert_eq!(count, 3);
                assert_eq!(error.as_deref(), Some("the reason"));
            }
            other => panic!("expected the fill dialog, got {:?}", other.is_some()),
        }
    }

    #[test]
    fn a_formula_the_engine_refuses_is_shown_and_never_written() {
        let mut session = session();
        let mut ai = ai();
        let request = Request::Formula { question: "add".into(), cell: "B2".into(), context: String::new() };
        let outcome = Outcome::Formula { formula: "=SUM(1;2)".into() };
        assert_eq!(ai.apply(&mut session, &request, Apply::Formula { cell: CellRef::new(1, 1) }, outcome), Ok(false));
        assert_eq!(undo_steps(&session), 0, "nothing was written");
        assert_eq!(value(&mut session, "B2"), Value::Null);
        assert!(matches!(ai.dialog, Some(Dialog::Formula { error: Some(_), .. })), "the dialog keeps the engine's reason");
    }

    #[test]
    fn a_series_name_becomes_a_sheet_name_the_workbook_accepts() {
        assert_eq!(sheet_name("Budget 2026"), "Budget 2026");
        assert_eq!(sheet_name("Q1[2026]: *"), "Q12026");
        assert_eq!(sheet_name("   "), "AI");
        assert_eq!(sheet_name(&"x".repeat(40)).len(), 31);
    }

    #[test]
    fn values_that_start_with_equals_are_moved_out_of_set_values() {
        let start = CellRef::new(0, 0);
        let matrix = vec![vec![json!("=1+1")], vec![json!("ok")]];
        let (kept, post) = split_equals(matrix, start);
        assert_eq!(kept, vec![vec![Value::Null], vec![json!("ok")]]);
        assert_eq!(post, vec![(CellRef::new(0, 0), "=1+1".to_string())]);
    }

    /// The mock a UI test would inject: the trait is public, the answer is fixed.
    struct Canned(&'static str);

    impl Backend for Canned {
        fn stream(&self, _: &Connection, _: &[Msg], on_delta: &mut dyn FnMut(&str)) -> Result<String, String> {
            on_delta(self.0);
            Ok(self.0.to_string())
        }

        fn models(&self, _: &Connection) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn a_canned_backend_feeds_the_same_pipeline() {
        // Exercises the seam the window uses: stream -> extract_json -> interpret -> apply.
        let backend: Arc<dyn Backend> = Arc::new(Canned(r#"{"title":"Budget","columns":["Item","Cost"],"rows":[["Rent",1200]]}"#));
        let connection = la::Connection {
            id: "test".into(),
            provider: "openai".into(),
            label: "Test".into(),
            base_url: "http://127.0.0.1:9".into(),
            model: "m".into(),
        };
        let request = Request::GenerateSheet { topic: "Budget".into(), rows: 4 };
        let messages = localsheets_ai::actions::messages(&request, "en").unwrap();
        let mut pieces = String::new();
        let answer = backend.stream(&connection, &messages, &mut |piece| pieces.push_str(piece)).unwrap();
        assert_eq!(pieces, answer);
        let outcome = localsheets_ai::actions::interpret(&request, &answer).unwrap();
        let mut session = session();
        assert_eq!(ai().apply(&mut session, &request, Apply::Sheet { preset: Preset::None }, outcome), Ok(true));
        assert_eq!(value(&mut session, "A2"), json!("Rent"));
    }
}
