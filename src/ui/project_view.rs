use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::Sense;
use egui_extras::Column;
use tracing::{debug, error, info, trace, warn};

use super::ui_types::App;

impl App {
    pub fn show_project_view(&mut self, ctx: &egui::Context) {
        egui::panel::SidePanel::left("printer_list")
            .min_width(200.)
            // .max_width(printer_list_size)
            .resizable(true)
            .show(ctx, |ui| {
                self.project_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Sync Projects").clicked() {
                self.cmd_tx
                    .as_ref()
                    .unwrap()
                    .send(crate::conn_manager::PrinterConnCmd::SyncProjects)
                    .unwrap();
            }
            //
        });

        //
    }

    fn project_list(&mut self, ui: &mut egui::Ui) {
        let mut table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            // .column(Column::initial(100.0).range(40.0..=300.0))
            // .column(Column::initial(100.0).at_least(40.0).clip(true))
            // .column(Column::remainder())
            // .min_scrolled_height(0.0)
            // .max_scroll_height(available_height)
            .sense(Sense::click());

        /// Columns:
        /// Thumbnail
        /// Printer
        /// Name
        /// Status ?
        /// Time
        /// Material
        /// Plate
        /// Time
        table
            .header(40., |mut header| {
                header.col(|ui| {
                    ui.strong("Thumbnail");
                });
                header.col(|ui| {
                    ui.strong("Printer");
                });
                header.col(|ui| {
                    ui.strong("Name");
                });
                header.col(|ui| {
                    ui.strong("Status");
                });
                header.col(|ui| {
                    ui.strong("Time");
                });
                header.col(|ui| {
                    ui.strong("Material");
                });
                header.col(|ui| {
                    ui.strong("Plate");
                });
                header.col(|ui| {
                    ui.strong("Time");
                });
            })
            .body(|mut body| {
                //
            });
    }
}
