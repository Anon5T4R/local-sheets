//! Every dialog and window of the app fits the window's minimum width (`with_min_inner_size` in
//! `apps/localsheets/src/main.rs`), with a workbook open and filled; the top bar, the command
//! palette and the update card are measured too. The bounding boxes come from the accesskit tree,
//! as `every_dialog_fits_the_narrow_window` does in the Local Keys suite.

use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use localsheets_engine::Session;
use localsheets_ui_egui::SheetApp;
use serde_json::json;

use crate::common::{Harness, app, builder};

/// The app's minimum inner width (`apps/localsheets/src/main.rs`).
const MIN_WIDTH: f32 = 640.0;

/// A workbook with data, a chart, a comment, a name and a second sheet: the dialogs draw their
/// lists and previews instead of their empty states.
fn filled_session() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s.execute(
        "range.setValues",
        json!({"range": "A1", "values": [["Quarter", "North", "South"], ["Q1", 1200, 980], ["Q2", 1500, 1310], ["Q3", 1730, 1605]], "parse": false}),
    )
    .unwrap();
    s.execute("insert.chart", json!({"range": "A1:C4", "type": "column", "at": "E3", "width": 320, "height": 220})).unwrap();
    s.execute("review.newComment", json!({"cell": "B2", "text": "Revisar o total do trimestre.", "author": "QA"})).unwrap();
    s.execute("formulas.defineName", json!({"name": "Vendas", "refersTo": "=Sheet1!$A$1:$C$4"})).unwrap();
    s.execute("home.insertSheet", json!({"name": "Resumo"})).unwrap();
    s.execute("sheet.activate", json!({"sheet": 0})).unwrap();
    s
}

fn harness_at(session: Session, width: f32) -> Harness {
    let mut h = builder().with_size(egui::vec2(width, 560.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app(session),
    );
    h.run_steps(4);
    h
}

/// Every accesskit node of `root`'s subtree with a box stays inside the window's width.
fn assert_subtree_inside(root: &egui_kittest::Node<'_>, what: &str, width: f32) {
    for node in root.children_recursive() {
        // A field's text runs past its own box (egui clips the drawing): the field is the node.
        if node.accesskit_node().role() == Role::TextRun {
            continue;
        }
        let Some(rect) = node.accesskit_node().bounding_box() else { continue };
        if rect.width() > 0.0 {
            assert!(
                rect.x0 >= -0.5 && rect.x1 <= f64::from(width) + 0.5,
                "{what}: {:?} outside the {width} pt window: {rect:?}",
                node.accesskit_node().role()
            );
        }
    }
}

/// Every node below the ribbon's scrollable rows (the grid, the tabs, the status bar and any
/// foreground card drawn outside a window) fits the window's width.
fn assert_chrome_inside(harness: &Harness, what: &str, width: f32) {
    for node in harness.root().children_recursive() {
        let Some(rect) = node.accesskit_node().bounding_box() else { continue };
        // The ribbon scrolls its card and its tab row horizontally by design (a narrow window
        // shows as many tabs as fit and the arrows scroll the rest); everything below it must fit.
        if rect.y1 <= 205.0 || node.accesskit_node().role() == Role::TextRun {
            continue;
        }
        if rect.width() > 0.0 {
            assert!(
                rect.x0 >= -0.5 && rect.x1 <= f64::from(width) + 0.5,
                "{what}: {:?} outside the {width} pt window: {rect:?}",
                node.accesskit_node().role()
            );
        }
    }
}

/// The windows on screen (the app's dialogs, the palette, the update card) fit the window width.
fn assert_windows_inside(harness: &Harness, what: &str, width: f32) -> usize {
    let mut seen = 0;
    for node in harness.query_all_by_role(Role::Window) {
        if node.accesskit_node().bounding_box().is_none() {
            continue;
        }
        seen += 1;
        assert_subtree_inside(&node, what, width);
    }
    seen
}

/// The dialogs that draw their own window (pickers and the macro step editor, which is its own
/// resizable window with its own test, are not dialogs of the app's chrome).
fn dialogs() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("insertCells", json!({})),
        ("deleteCells", json!({})),
        ("insertPaste", json!({})),
        ("arrangeAll", json!({})),
        ("rowHeight", json!({})),
        ("columnWidth", json!({})),
        ("defaultWidth", json!({})),
        ("renameSheet", json!({})),
        ("moveSheet", json!({})),
        ("protectSheet", json!({})),
        ("allowEditRanges", json!({})),
        ("protectWorkbook", json!({})),
        ("unprotectWorkbook", json!({})),
        ("openPassword", json!({})),
        ("unprotectSheet", json!({})),
        ("unhideSheet", json!({})),
        ("pasteSpecial", json!({})),
        ("series", json!({})),
        ("comment", json!({})),
        ("openLink", json!({"url": "https://example.org/longo/caminho/do/link"})),
        ("insertLink", json!({})),
        ("sparkline", json!({})),
        ("headerFooter", json!({})),
        ("pageSetup", json!({})),
        ("textToColumns", json!({})),
        ("removeDuplicates", json!({})),
        ("advancedFilter", json!({})),
        ("goalSeek", json!({})),
        ("forecastSheet", json!({})),
        ("analysisToolPak", json!({})),
        ("solver", json!({})),
        ("solverResults", json!({"result": "Solve", "value": 42})),
        ("subtotal", json!({})),
        ("outlineSettings", json!({})),
        ("zoom", json!({})),
        ("defineName", json!({})),
        ("tableResize", json!({})),
        ("dataValidation", json!({})),
        // The conditional-formatting quick forms: one dialog (`cfQuick`) with a type parameter.
        ("cfQuick", json!({"type": "cellIs", "operator": "between"})),
        ("cfQuick", json!({"type": "containsText"})),
        ("cfQuick", json!({"type": "timePeriod"})),
        ("formatCells", json!({})),
        ("insertFunction", json!({})),
        ("find", json!({})),
        ("goTo", json!({})),
        ("pivotCalculatedField", json!({})),
        ("goToSpecial", json!({})),
        ("sort", json!({})),
        ("nameManager", json!({})),
        ("pasteName", json!({})),
        ("manageRules", json!({})),
        ("agents", json!({})),
        ("about", json!({})),
        ("recommendedCharts", json!({})),
        ("journal", json!({})),
        ("macros", json!({})),
        ("recordMacro", json!({})),
        ("statistics", json!({})),
        ("accessibility", json!({})),
        ("errorChecking", json!({})),
        ("evaluateFormula", json!({})),
        ("spelling", json!({})),
        ("chartTitle", json!({})),
        ("pickList", json!({})),
        ("saveCopy", json!({"format": "xlsx"})),
        ("saveChanges", json!({"title": "Notas do trimestre.xlsx", "quit": true})),
        ("start", json!({})),
    ]
}

/// Every dialog fits the window's minimum width, with the workbook filled.
#[test]
fn every_dialog_fits_the_narrow_window() {
    for (name, params) in dialogs() {
        let mut h = harness_at(filled_session(), MIN_WIDTH);
        let chart = h.state().session.active().and_then(|d| d.wb.active().and_then(|s| s.charts.first().map(|c| c.id)));
        let mut params = params;
        if let (Some(id), Some(object)) = (chart, params.as_object_mut()) {
            object.insert("chart".into(), json!(id));
        }
        h.state_mut().open_dialog(name, params);
        h.run_steps(4);
        assert_eq!(h.state().dialog.as_ref().map(|d| d.name.as_str()), Some(name), "{name} did not open");
        assert!(assert_windows_inside(&h, name, MIN_WIDTH) >= 1, "{name} draws no window");
    }
}

/// The top bar (title, file name and the suite's buttons) fits the minimum width. The ribbon below
/// it scrolls its tab row horizontally (a narrow window shows as many tabs as fit), so it is not
/// part of this row.
#[test]
fn the_top_bar_fits_the_narrow_window() {
    let h = harness_at(filled_session(), MIN_WIDTH);
    for node in h.root().children_recursive() {
        let Some(rect) = node.accesskit_node().bounding_box() else { continue };
        if rect.y1 > 46.0 || node.accesskit_node().role() == Role::TextRun {
            continue;
        }
        assert!(rect.x0 >= -0.5 && rect.x1 <= f64::from(MIN_WIDTH) + 0.5, "top bar: {:?} outside the window: {rect:?}", node.accesskit_node().role());
    }
    // The suite's three buttons are the last ones, inside the window.
    for label in ["Search commands", "Theme: Light", "Settings"] {
        let rect = h.get_by_label(label).rect();
        assert!(rect.max.x <= MIN_WIDTH, "{label} ends at {}", rect.max.x);
    }
}

/// The side panes the app opens in its own window (Comments, Watch, Selection, Format Chart, AI,
/// Pivot) fit the minimum width.
#[test]
fn the_panes_fit_the_narrow_window() {
    for pane in ["comments", "watch", "selectionPane", "formatChart"] {
        let mut h = harness_at(filled_session(), MIN_WIDTH);
        h.state_mut().open_dialog(pane, json!({}));
        h.run_steps(3);
        assert!(h.state().dialog.is_none(), "{pane} is a pane, not a dialog");
        assert_chrome_inside(&h, pane, MIN_WIDTH);
    }
}

/// The command palette (`commandSearch`) fits the minimum width.
#[test]
fn the_palette_fits_the_narrow_window() {
    let mut h = harness_at(filled_session(), MIN_WIDTH);
    h.state_mut().open_dialog("commandSearch", json!({}));
    h.run_steps(3);
    assert!(h.query_by_label("Search commands").is_some(), "the palette's search field");
    assert!(assert_windows_inside(&h, "commandSearch", MIN_WIDTH) >= 1, "the palette draws no window");
}

/// The update card (`craft-update`, an offered release) fits the minimum width.
#[test]
fn the_update_card_fits_the_narrow_window() {
    let mut h = harness_at(filled_session(), MIN_WIDTH);
    let release = craft_update::Release { tag: "v9.9.9".into(), version: craft_update::version::parse("9.9.9"), ..craft_update::Release::default() };
    h.state_mut().offer_update(release);
    h.run_steps(4);
    h.query_by_label("LATER").expect("the offered card");
    assert_chrome_inside(&h, "update card", MIN_WIDTH);
}
