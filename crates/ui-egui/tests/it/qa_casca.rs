//! Visual QA of the suite's shell (title bar buttons, Settings, command search): renders PNGs for
//! review at the window's minimum width and at 1100 px. Run with
//! `cargo test -p localsheets-ui-egui --test it qa_casca -- --ignored --nocapture`; the images go
//! to `target/qa-casca/` at the workspace root.

use egui_kittest::kittest::Queryable as _;
use localsheets_engine::Session;
use localsheets_ui_egui::SheetApp;
use serde_json::json;

/// The app's minimum inner width (`apps/localsheets/src/main.rs`).
const MIN_WIDTH: f32 = 640.0;

fn app(dark: bool) -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [["Item", "Cost"], ["Rent", 1200]], "parse": false})).unwrap();
    let mut app = crate::common::app(session);
    app.ui.dark = dark;
    app.ui.system_theme = false;
    app
}

fn render(app: SheetApp, width: f32, name: &str, setup: impl FnOnce(&mut egui_kittest::Harness<'static, SheetApp>)) {
    let mut h = crate::common::builder().with_size(egui::vec2(width, 560.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(5);
    setup(&mut h);
    h.run_steps(5);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/qa-casca");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("sheets-{name}-{}.png", width as u32));
    h.render().expect("render").save(&path).expect("save");
    println!("{}", path.display());
}

#[test]
#[ignore = "visual QA: renders PNGs for review"]
fn renders_the_shell_for_review() {
    for width in [MIN_WIDTH, 1100.0] {
        for (name, dark) in [("light", false), ("dark", true)] {
            render(app(dark), width, name, |_| {});
        }
        render(app(false), width, "settings", |h| {
            h.get_by_label("Settings").click();
        });
        render(app(false), width, "search", |h| {
            h.get_by_label("Search commands").click();
        });
        render(app(false), width, "help", |h| {
            h.state_mut().ui.ribbon_tab = "Help".into();
        });
    }
}
