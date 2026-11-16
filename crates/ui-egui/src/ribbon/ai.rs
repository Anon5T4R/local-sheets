//! The AI tab: the three sheet actions and the connections panel.

use super::{Icon, SheetApp, Ui, big_button};

pub(super) fn ai(app: &mut SheetApp, ui: &mut Ui) {
    if big_button(ui, Icon::Sheet, "Generate\nSheet", "Ask the AI for a new sheet from a description", false).clicked() {
        app.ai.open_generate();
    }
    if big_button(ui, Icon::FillDown, "Fill\nColumn", "Fill the current column from the neighbouring columns", false).clicked() {
        app.ai.open_fill(&app.session);
    }
    if big_button(ui, Icon::Function, "Formula", "Ask for the current cell's formula in plain language", false).clicked() {
        app.ai.open_formula(&app.session);
    }
    if big_button(ui, Icon::Settings, "Connections", "AI providers, keys and models", false).clicked() {
        app.ai.panel = true;
    }
}
