use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::Vec2;

use crate::{config::PrinterConfig, conn_manager::PrinterConnCmd, ui::ui_types::App};

impl App {
    // #[cfg(feature = "nope")]
    pub fn show_printers_config(&mut self, ctx: &egui::Context) {
        let printer_list_size = 150.;

        let prev_selected = self.options.selected_printer.clone();
        egui::panel::SidePanel::left("printer_list")
            .min_width(printer_list_size)
            // .max_width(printer_list_size)
            .resizable(true)
            .show(ctx, |ui| {
                let row_height = 30.;

                let num_rows = self.config.printers().len();

                egui::ScrollArea::vertical()
                    .max_width(printer_list_size)
                    .auto_shrink(false)
                    .show_rows(ui, row_height, num_rows, |ui, row_range| {
                        for row in row_range {
                            let printer = &self.config.printers()[row];
                            /// XXX: will this block?
                            let printer = printer.blocking_read();
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

        if prev_selected != self.options.selected_printer {
            self.options.selected_printer_cfg = None;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.add(egui::Button::new("Sync Printers")).clicked() {
                self.cmd_tx
                    .as_ref()
                    .unwrap()
                    .send(PrinterConnCmd::SyncPrinters)
                    .unwrap();
                //
                self.printer_config_page.syncing_printers = Some(false);
            }

            match self.printer_config_page.syncing_printers {
                Some(true) => {
                    ui.label("Syncing printers...");
                }
                Some(false) => {
                    ui.label("Syncing printers... Done");
                    // self.printer_config_page.syncing_printers = None;
                }
                _ => {}
            }

            self.show_printer_config(ui);
        });
    }

    fn show_printer_config(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.options.selected_printer.as_ref().cloned() else {
            ui.label("No printer selected");
            return;
        };

        if self.options.selected_printer_cfg.is_none() {
            if let Some(cfg) = self.config.get_printer(&id) {
                // self.options.selected_printer_cfg = Some((*cfg).clone());
                let cfg = cfg.blocking_read();
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

        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("printer_setting_grid")
                // .spacing(spacing)
                // .striped(true)
                .num_columns(2)
                // .min_col_width(min_col_width)
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut cfg.name);
                        ui.allocate_space(ui.available_size());
                    });
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
            // ui.allocate_space(Vec2::new(ui.available_size_before_wrap().x, 0.));
        });

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("Graph Values");
            //
        });

        // let mut printer = self.config
    }
}
