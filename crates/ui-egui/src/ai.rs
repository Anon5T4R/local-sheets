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
use localsheets_ai::{AI_APP_ID, AI_APP_URL, Backend, LiveBackend, Outcome, Request};
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
pub(crate) enum Apply {
    /// A new sheet (the outcome carries the cells).
    Sheet,
    /// The target column: where it starts and the A1 range the values go into.
    Fill { start: CellRef, range: String },
    /// The cell the formula goes into.
    Formula { cell: CellRef },
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
    Generate { topic: String, rows: u32, error: Option<String> },
    Fill { header: String, context: String, target: String, start: CellRef, count: u32, single: bool, error: Option<String> },
    Formula { question: String, cell: CellRef, context: String, formula: Option<String>, error: Option<String> },
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
            Ok(outcome) => match self.apply(session, &running.request, running.apply, outcome) {
                Ok(true) => true,
                Ok(false) => false,
                Err(error) => {
                    self.error = Some(error);
                    false
                }
            },
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
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
            (Request::GenerateSheet { .. }, Apply::Sheet, Outcome::Sheet { title, columns, rows }) => {
                session
                    .grouped("AI: generate sheet", |s| {
                        let inserted = match s.execute("home.insertSheet", json!({ "name": sheet_name(&title) })) {
                            Ok(value) => value,
                            // A name that clashes or is invalid falls back to the next free one.
                            Err(_) => s.execute("home.insertSheet", json!({}))?,
                        };
                        let index = inserted.get("index").and_then(Value::as_u64).unwrap_or(0);
                        let mut matrix: Vec<Vec<Value>> = Vec::with_capacity(rows.len() + 1);
                        matrix.push(columns.into_iter().map(Value::String).collect());
                        matrix.extend(rows);
                        let (matrix, post) = split_equals(matrix, CellRef::new(0, 0));
                        s.execute("range.setValues", json!({ "sheet": index, "range": "A1", "values": matrix, "parse": false }))?;
                        for (cell, text) in post {
                            s.execute("cell.set", json!({ "sheet": index, "cell": cell.a1(), "input": format!("'{text}") }))?;
                        }
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
                Ok(true)
            }
            (Request::FillColumn { .. }, Apply::Fill { start, range }, Outcome::Values { values }) => {
                session
                    .grouped("AI: fill column", |s| {
                        let matrix: Vec<Vec<Value>> = values.into_iter().map(|value| vec![value]).collect();
                        let (matrix, post) = split_equals(matrix, start);
                        s.execute("range.setValues", json!({ "range": range, "values": matrix, "parse": false }))?;
                        for (cell, text) in post {
                            s.execute("cell.set", json!({ "cell": cell.a1(), "input": format!("'{text}") }))?;
                        }
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

    pub(crate) fn open_generate(&mut self) {
        self.dialog = Some(Dialog::Generate { topic: String::new(), rows: 20, error: None });
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
    };
    egui::Window::new(title).collapsible(false).resizable(false).default_width(460.0).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(
        ctx,
        |ui| match &mut dialog {
            Dialog::Generate { topic, rows, error: dialog_error } => {
                ui.label(language.tr("Describe the sheet: a topic, or a whole text to organize."));
                ui.add(egui::TextEdit::multiline(topic).desired_rows(4).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.label(language.tr("Rows"));
                    ui.add(egui::DragValue::new(rows).range(1..=200));
                });
                if let Some(message) = dialog_error.as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
                ui.horizontal(|ui| {
                    if ui.button(language.tr("Generate")).clicked() {
                        run = Some((Request::GenerateSheet { topic: topic.clone(), rows: *rows }, Apply::Sheet));
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
            Dialog::Generate { error: dialog_error, .. } | Dialog::Fill { error: dialog_error, .. } | Dialog::Formula { error: dialog_error, .. } => {
                *dialog_error = Some(message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            columns: vec!["Item".into(), "Cost".into()],
            rows: vec![vec![json!("Rent"), json!(1200.0)], vec![json!("Food"), json!(400.0)]],
        };
        assert_eq!(ai.apply(&mut session, &request, Apply::Sheet, outcome), Ok(true));
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
        let outcome = Outcome::Values { values: vec![json!("=SUM(A1:A2)"), json!("plain")] };
        let start = CellRef::new(0, 0);
        let range = range_a1(start, 2);
        assert_eq!(ai.apply(&mut session, &request, Apply::Fill { start, range }, outcome), Ok(true));
        // The command would read a leading `=` as a formula: the text is written as text instead.
        assert_eq!(value(&mut session, "A1"), json!("=SUM(A1:A2)"));
        assert_eq!(value(&mut session, "A2"), json!("plain"));
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
        assert_eq!(ai().apply(&mut session, &request, Apply::Sheet, outcome), Ok(true));
        assert_eq!(value(&mut session, "A2"), json!("Rent"));
    }
}
