use anyhow::{anyhow, bail, ensure, Context, Result};
use egui_phosphor::fill;
use tracing::{debug, error, info, trace, warn};

use crate::ui_types::{App, GridLocation};

/// display
impl App {
    pub fn show_options(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO: Options");

        // egui::widgets::global_dark_light_mode_buttons(ui);

        // ui.separator();

        egui::Grid::new("options_grid").show(ui, |ui| {
            ui.label("Rows");

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_UP)),
                )
                .clicked()
            {
                self.change_rows(false);
            }
            ui.add_sized(
                ui.available_size(),
                egui::Label::new(&format!("{:?}", self.options.dashboard_size.1)),
            );

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_DOWN)),
                )
                .clicked()
            {
                self.change_rows(true);
            }

            ui.end_row();

            ui.label("Columns");
            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_LEFT)),
                )
                .clicked()
            {
                self.change_columns(false);
            }
            ui.add_sized(
                ui.available_size(),
                egui::Label::new(&format!("{:?}", self.options.dashboard_size.0)),
            );

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_RIGHT)),
                )
                .clicked()
            {
                self.change_columns(true);
            }

            ui.end_row();
        });
    }
}

/// apply options
impl App {
    pub fn move_printer(&mut self, from: &GridLocation, to: &GridLocation) {
        if from == to {
            return;
        }
        match (
            self.printer_order.remove(&from),
            self.printer_order.remove(&to),
        ) {
            (Some(id_from), Some(id_to)) => {
                debug!("TODO: swap printers");
            }
            (Some(id), None) => {
                debug!("moving printer {} from {:?} to {:?}", id, from, to);
                self.printer_order.insert(*to, id);
                //
            }
            (None, _) => {
                error!("Drop: No printer at drop source");
                // self.printer_order.insert(to, from);
                // self.printer_order.remove(&from);
            }
        }
    }

    pub fn change_rows(&mut self, add: bool) {
        if add {
            // self.options.dashboard_size.1 += 1;
        } else {
            if self.options.dashboard_size.1 == 1 {
                return;
            }
            warn!("TODO: what to do if printer is in removed row?");

            for x in 0..self.options.dashboard_size.0 {
                let pos = GridLocation::new(x, self.options.dashboard_size.1 - 1);
            }

            self.options.dashboard_size.1 -= 1;
        }
    }

    pub fn change_columns(&mut self, add: bool) {
        if add {
            self.options.dashboard_size.0 += 1;
        } else {
            if self.options.dashboard_size.0 == 1 {
                return;
            }
            warn!("TODO: what to do if printer is in removed column?");
            self.options.dashboard_size.0 -= 1;
        }
    }
}
