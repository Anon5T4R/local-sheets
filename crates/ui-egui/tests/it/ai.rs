//! The AI tab in the window: the four entries are reachable, the two that need only a workbook open
//! their dialog, and the panel opens from the ribbon.

use egui_kittest::kittest::Queryable as _;
use localsheets_engine::Session;
use localsheets_ui_egui::SheetApp;
use localsheets_ui_egui::ai::Dialog;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Default::default());
    app.ui.ribbon_tab = "AI".into();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

#[test]
fn the_ai_tab_offers_the_three_actions_and_connections() {
    let h = harness();
    for label in ["Generate Sheet", "Fill Column", "Formula", "Connections"] {
        assert!(h.query_by_label(label).is_some(), "{label} is missing from the AI tab");
    }
}

#[test]
fn generate_opens_its_dialog() {
    let mut h = harness();
    h.get_by_label("Generate Sheet").click();
    h.run_steps(3);
    assert!(matches!(h.state().ai.dialog, Some(Dialog::Generate { .. })), "the generate dialog is open");
    // The dialog's own labels: the description field and the row count.
    assert!(h.query_by_label("Describe the sheet: a topic, or a whole text to organize.").is_some());
    assert!(h.query_by_label("Generate").is_some(), "the dialog's own button is painted");
}

#[test]
fn fill_column_opens_its_dialog_for_the_selection() {
    let mut h = harness();
    h.get_by_label("Fill Column").click();
    h.run_steps(3);
    assert!(matches!(h.state().ai.dialog, Some(Dialog::Fill { .. })), "the fill dialog is open");
    assert!(h.query_by_label("Fill the current column from the neighbouring columns.").is_some());
}

#[test]
fn connections_opens_the_panel() {
    let mut h = harness();
    h.get_by_label("Connections").click();
    h.run_steps(3);
    assert!(h.state().ai.panel, "the connections panel is open");
    assert!(h.query_by_label("Add connection").is_some(), "the panel offers a new connection");
}
