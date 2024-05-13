use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::ui_types::App;

impl App {
    pub fn show_printers_config(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO: Printer Config");
    }
}
