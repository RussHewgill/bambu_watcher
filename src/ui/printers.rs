use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{config::PrinterConfig, conn_manager::PrinterConnCmd, ui_types::App};

impl App {
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
