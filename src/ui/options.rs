use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::ui_types::App;

impl App {
    pub fn show_options(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO: Options");

        egui::widgets::global_dark_light_mode_buttons(ui);

        ui.separator();

        egui::Grid::new("options_grid").show(ui, |ui| {
            ui.label("Rows");

            if ui.button("🔽").clicked() {
                self.options.dashboard_size.1 += 1;
            }
            ui.label(&format!("{:?}", self.options.dashboard_size.1));
            if ui.button("🔼").clicked() {
                self.options.dashboard_size.1 += 1;
            }

            ui.end_row();

            ui.label("Columns");

            if ui.button("◀").clicked() {
                self.options.dashboard_size.0 += 1;
            }
            ui.label(&format!("{:?}", self.options.dashboard_size.0));
            if ui.button("▶").clicked() {
                self.options.dashboard_size.0 += 1;
            }

            ui.end_row();
        });
    }
}
