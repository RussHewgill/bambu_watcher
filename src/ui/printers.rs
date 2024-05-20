use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{config::PrinterConfig, conn_manager::PrinterConnCmd, ui::ui_types::App};

impl App {
    // #[cfg(feature = "nope")]
    pub fn show_printers_config(&mut self, ctx: &egui::Context) {
        let printer_list_size = 200.;
        egui::panel::SidePanel::left("printer_list")
            .min_width(printer_list_size)
            .max_width(printer_list_size)
            .show(ctx, |ui| {
                let row_height = 30.;

                let num_rows = self.config.printers().len();

                egui::ScrollArea::vertical()
                    .max_width(printer_list_size)
                    .auto_shrink(false)
                    .show_rows(ui, row_height, num_rows, |ui, row_range| {
                        for row in row_range {
                            let printer = &self.config.printers()[row];
                            let name = &printer.name;
                            let id = &printer.serial;
                            // ui.label(name);
                            ui.selectable_value(
                                &mut self.options.selected_printer,
                                Some(id.clone()),
                                &format!("{}", name),
                            );
                        }
                        ui.allocate_space(ui.available_size());
                        //
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(id) = self.options.selected_printer.as_ref().cloned() else {
                ui.label("No printer selected");
                return;
            };

            if self.options.selected_printer_cfg.is_none() {
                if let Some(cfg) = self.config.get_printer(&id) {
                    // self.options.selected_printer_cfg = Some((*cfg).clone());
                    self.options.selected_printer_cfg = Some(cfg.clone());
                } else {
                    ui.label("Printer not found");
                    return;
                }
            }

            let Some(cfg) = self.options.selected_printer_cfg.as_mut() else {
                ui.label("No printer selected");
                return;
            };

            egui::Grid::new("printer_setting_grid").show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut cfg.name);
                ui.end_row();

                ui.label("Host");
                ui.text_edit_singleline(&mut cfg.host);
                ui.end_row();

                ui.label("Access Code");
                ui.text_edit_singleline(&mut cfg.access_code);
                ui.end_row();
            });

            if ui.button("Save").clicked() {
                //
            }

            // let mut printer = self.config

            //
        });
    }

    #[cfg(feature = "nope")]
    pub fn show_printers_config(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO: Printer Config");

        egui::Grid::new("printer_config_grid").show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.new_printer.name);
            ui.end_row();

            ui.label("Host");
            ui.text_edit_singleline(&mut self.new_printer.host);
            ui.end_row();

            ui.label("Access Code");
            ui.text_edit_singleline(&mut self.new_printer.access_code);
            ui.end_row();

            ui.label("Serial");
            ui.text_edit_singleline(&mut self.new_printer.serial);
            ui.end_row();
        });

        if ui.button("Add").clicked() {
            self.cmd_tx
                .as_ref()
                .expect("cmd_tx not set")
                .try_send(PrinterConnCmd::AddPrinter(PrinterConfig {
                    name: self.new_printer.name.clone(),
                    host: self.new_printer.host.clone(),
                    access_code: self.new_printer.access_code.clone(),
                    serial: self.new_printer.serial.clone(),
                }))
                .unwrap();
            self.unplaced_printers.push(self.new_printer.serial.clone());
        }
    }
}
