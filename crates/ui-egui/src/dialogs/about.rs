use super::Dialog;

/// Help ▸ About Local Sheets: the app, its contributors and the AI models that helped. The credits
/// are compiled in (`crate::credits`, docs/contributors.md).
pub(super) fn about(ui: &mut egui::Ui, d: &mut Dialog) {
    ui.horizontal(|ui| {
        for tab in ["About", "Contributors", "Models"] {
            if ui.selectable_label(d.tab == tab, tab).clicked() {
                d.tab = tab.to_string();
            }
        }
    });
    ui.separator();
    match d.tab.as_str() {
        "Contributors" => crate::credits::contributors_ui(ui),
        "Models" => crate::credits::models_ui(ui),
        _ => {
            ui.heading("Local Sheets");
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.add_space(4.0);
            ui.label("A clean-room, open-source, Rust-native spreadsheet. Not affiliated with Microsoft.");
            ui.hyperlink_to("github.com/Anon5T4R/local-sheets", "https://github.com/Anon5T4R/local-sheets");
            ui.hyperlink_to("Report an issue", "https://github.com/Anon5T4R/local-sheets/issues");
        }
    }
}
