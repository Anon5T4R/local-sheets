//! Saved appearance preferences and live OS changes through the whole UI.

use egui::{Theme, ThemePreference};
use egui_kittest::kittest::Queryable as _;
use localsheets_engine::Session;
use localsheets_ui_egui::{SheetApp, UiState, control, theme::Tokens};
use serde_json::{Value, json};

fn harness(ui: UiState) -> egui_kittest::Harness<'static, SheetApp> {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = crate::common::app(session);
    app.ui = ui;
    let mut h = crate::common::builder().with_size(egui::vec2(1200.0, 800.0)).with_step_dt(1.0 / 60.0).build_ui_state(
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

fn preferences(h: &egui_kittest::Harness<'static, SheetApp>) -> Value {
    serde_json::to_value(&h.state().ui).unwrap()
}

fn system_theme(h: &mut egui_kittest::Harness<'static, SheetApp>, theme: Option<Theme>) {
    h.input_mut().system_theme = theme;
    h.run_steps(2);
}

fn set_mode(h: &mut egui_kittest::Harness<'static, SheetApp>, mode: &str) {
    let ctx = h.ctx.clone();
    let control::Outcome::Done(response) =
        control::handle(h.state_mut(), &ctx, "engine.execute", &json!({"command": "view.theme", "params": {"mode": mode}}))
    else {
        panic!("engine.execute should return a response");
    };
    assert_eq!(response["ok"], true, "{response}");
    h.run_steps(2);
}

fn select_ribbon_theme(h: &mut egui_kittest::Harness<'static, SheetApp>, label: &str) {
    // The View tab's first combo box is the display theme (the interface language follows it).
    h.get_all_by_role(egui::accesskit::Role::ComboBox).next().expect("display theme combo").click();
    h.run_steps(2);
    h.get_by_role_and_label(egui::accesskit::Role::Button, label).click();
    h.run_steps(2);
}

fn assert_appearance(h: &egui_kittest::Harness<'static, SheetApp>, dark: bool) {
    let visuals = &h.ctx.global_style().visuals;
    let tokens = if dark { Tokens::dark() } else { Tokens::light() };
    assert_eq!(visuals.dark_mode, dark);
    assert_eq!(visuals.panel_fill, tokens.window);
    assert_eq!(visuals.window_fill, tokens.menu_bg);
    assert_eq!(visuals.selection.bg_fill, tokens.accent_soft);
}

#[test]
fn old_preferences_stay_manual_and_system_preference_survives_reload() {
    for dark in [false, true] {
        let ui: UiState = serde_json::from_value(json!({"dark": dark})).unwrap();
        assert_eq!(ui.dark, dark);
        let mut h = harness(ui);
        assert_eq!(preferences(&h)["systemTheme"], false, "old saved settings remain manual");
        system_theme(&mut h, Some(if dark { Theme::Light } else { Theme::Dark }));
        assert_appearance(&h, dark);
    }

    let mut h = harness(UiState::default());
    set_mode(&mut h, "system");
    system_theme(&mut h, Some(Theme::Dark));
    let saved = serde_json::to_string(&h.state().ui).unwrap();
    let mut restored = harness(serde_json::from_str(&saved).unwrap());
    assert_eq!(preferences(&restored)["systemTheme"], true);
    system_theme(&mut restored, Some(Theme::Light));
    assert_appearance(&restored, false);
    system_theme(&mut restored, Some(Theme::Dark));
    assert_appearance(&restored, true);
}

#[test]
fn system_tracks_live_os_changes_with_both_localsheets_palettes_and_light_fallback() {
    let mut h = harness(UiState::default());
    set_mode(&mut h, "system");
    assert_eq!(h.ctx.options(|o| o.theme_preference), ThemePreference::System);
    // egui switches between these two style slots when the OS appearance changes.
    for (theme, tokens) in [(Theme::Light, Tokens::light()), (Theme::Dark, Tokens::dark())] {
        let style = h.ctx.style_of(theme);
        assert_eq!(style.visuals.panel_fill, tokens.window);
        assert_eq!(style.visuals.window_fill, tokens.menu_bg);
        assert_eq!(style.visuals.selection.bg_fill, tokens.accent_soft);
    }
    for (reported, dark) in [(Some(Theme::Dark), true), (Some(Theme::Light), false), (Some(Theme::Dark), true), (None, false)] {
        system_theme(&mut h, reported);
        assert_appearance(&h, dark);
        assert_eq!(preferences(&h)["systemTheme"], true);
        assert_eq!(h.ctx.theme(), if dark { Theme::Dark } else { Theme::Light });
    }
}

#[test]
fn ribbon_selects_system_and_fixed_modes_ignore_subsequent_os_changes() {
    let mut h = harness(UiState::default());
    system_theme(&mut h, Some(Theme::Dark));
    assert_appearance(&h, false);
    h.state_mut().run("ui.ribbonTab", json!({"tab": "View"})).unwrap();
    h.run_steps(2);
    select_ribbon_theme(&mut h, "Automatic");
    assert_eq!(preferences(&h)["systemTheme"], true);
    assert_eq!(h.ctx.theme(), Theme::Dark);
    assert_appearance(&h, true);
    for (label, dark, preference) in [("Dark", true, ThemePreference::Dark), ("Light", false, ThemePreference::Light)] {
        select_ribbon_theme(&mut h, label);
        assert_eq!(preferences(&h)["systemTheme"], false);
        assert_eq!(h.ctx.options(|o| o.theme_preference), preference);
        for os in [Some(Theme::Dark), Some(Theme::Light), None] {
            system_theme(&mut h, os);
            assert_appearance(&h, dark);
        }
    }
}

#[test]
fn invalid_mode_preserves_preferences_and_legacy_overrides_leave_system_mode() {
    let mut h = harness(UiState::default());
    set_mode(&mut h, "system");
    system_theme(&mut h, Some(Theme::Dark));
    let before = preferences(&h);
    assert!(h.state_mut().run("view.theme", json!({"mode": "sepia"})).is_err());
    h.run_steps(2);
    assert_eq!(preferences(&h), before);
    assert_appearance(&h, true);

    // The old toggle flips the displayed appearance, even when System chose it.
    h.state_mut().run("view.darkMode", json!({})).unwrap();
    h.run_steps(2);
    assert_eq!(preferences(&h)["systemTheme"], false);
    assert_appearance(&h, false);

    set_mode(&mut h, "system");
    system_theme(&mut h, Some(Theme::Light));
    h.state_mut().run("view.darkMode", json!({"on": true})).unwrap();
    h.run_steps(2);
    assert_eq!(preferences(&h)["systemTheme"], false);
    assert_appearance(&h, true);

    set_mode(&mut h, "system");
    system_theme(&mut h, Some(Theme::Dark));
    let ctx = h.ctx.clone();
    let control::Outcome::Done(response) = control::handle(h.state_mut(), &ctx, "ui.set", &json!({"dark": false})) else {
        panic!("ui.set should return a response");
    };
    assert_eq!(response["ok"], true);
    h.run_steps(2);
    assert_eq!(preferences(&h)["systemTheme"], false);
    assert_appearance(&h, false);
    system_theme(&mut h, Some(Theme::Dark));
    assert_appearance(&h, false);
}

#[test]
fn title_bar_switch_and_file_menu_choose_the_theme() {
    // #194: one click in the title bar switches light/dark; File › Display theme offers all three.
    let mut h = harness(UiState::default());
    system_theme(&mut h, Some(Theme::Light));
    assert_appearance(&h, false);
    // The title bar's theme button cycles Automatic, Light, Dark; it stands on Light here.
    h.get_by_label("Theme: Light").click();
    h.run_steps(2);
    assert_eq!(preferences(&h)["dark"], true);
    assert_eq!(preferences(&h)["systemTheme"], false);
    assert_appearance(&h, true);
    h.get_by_label("Theme: Dark").click();
    h.run_steps(2);
    assert_eq!(preferences(&h)["systemTheme"], true);
    assert_appearance(&h, false);

    // File opens Backstage; its Options page holds the Office Theme choice (#331).
    h.get_by_label("File").click();
    h.run_steps(3);
    // The title bar's gear is also named Settings; the Backstage entry is the later one.
    h.get_all_by_role_and_label(egui::accesskit::Role::Button, "Settings").last().expect("the Backstage Settings entry").click();
    h.run_steps(3);
    // The first combo box on the page is Display theme.
    h.get_all_by_role(egui::accesskit::Role::ComboBox).next().expect("theme combo").click();
    h.run_steps(3);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Automatic").click();
    h.run_steps(3);
    assert_eq!(preferences(&h)["systemTheme"], true);
    system_theme(&mut h, Some(Theme::Dark));
    assert_appearance(&h, true);
}

/// The suite's title-bar controls: the theme button cycles Automatic, Light, Dark; the gear opens
/// File > Settings; the magnifier opens the command search and running a command from it works.
#[test]
fn title_bar_theme_button_gear_and_magnifier() {
    let mut h = harness(UiState::default());
    for (now, system, dark) in [("Light", false, true), ("Dark", true, true), ("Automatic", false, false)] {
        h.get_by_label(&format!("Theme: {now}")).click();
        h.run_steps(3);
        let prefs = preferences(&h);
        assert_eq!(prefs["systemTheme"], system, "after {now}");
        if !system {
            assert_eq!(prefs["dark"], dark, "after {now}");
        }
    }
    h.get_by_label("Search commands").click();
    h.run_steps(3);
    assert_eq!(h.state().dialog.as_ref().map(|d| d.name()), Some("commandSearch"), "the magnifier opens the command search");
    // A command picked in the search runs, and the search closes.
    // The search box has the focus when it opens.
    h.input_mut().events.push(egui::Event::Text("freeze top row".into()));
    h.run_steps(3);
    h.get_by_label_contains("Freeze Top Row").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert!(h.state().session.journal.iter().any(|(id, _)| id == "view.freezeTopRow"), "the picked command ran");
    assert!(h.state().backstage.is_none());
    h.get_by_label("Settings").click();
    h.run_steps(3);
    assert!(h.state().backstage.as_ref().is_some_and(|b| b.page.id() == "options"), "the gear opens File > Settings");
}
