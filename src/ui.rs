pub mod dashboard;
pub mod options;
pub mod plotting;
pub mod printers;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Align, Color32, Layout, Margin, Response, Rounding, Sense, Stroke, Vec2};
use egui_phosphor::fill;

use dashmap::DashMap;
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc, time::Duration};

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::{PrinterConnCmd, PrinterId},
    icons::*,
    status::{PrinterState, PrinterStatus},
    ui_types::{App, GridLocation, Tab},
};

/// new
impl App {
    pub fn new(
        // tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        // tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        config: ConfigArc,
        cc: &eframe::CreationContext<'_>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    ) -> Self {
        let mut out = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            // warn!("using default app state");
            // Self::default()
        } else {
            Self::default()
        };

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        // out.tray = tray_icon;
        out.printer_states = printer_states;
        out.config = config;
        // out.alert_tx = Some(alert_tx);

        out.cmd_tx = Some(cmd_tx);

        out.unplaced_printers = out
            .config
            .printers()
            .iter()
            .map(|p| p.serial.clone())
            .collect();
        /// for each printer that isn't in printer_order, queue to add
        for (_, id) in out.printer_order.iter() {
            out.unplaced_printers.retain(|p| p != id);
        }

        /// remove printers that were previously placed but are no longer in the config
        {
            let current_printers = out
                .config
                .printers()
                .iter()
                .map(|c| c.serial.clone())
                .collect::<HashSet<_>>();

            out.unplaced_printers
                .retain(|p| current_printers.contains(p));
            out.printer_order
                .retain(|_, v| current_printers.contains(v));
        }
        // for id in out.config.printers.iter() {
        //     out.unplaced_printers.retain(|p| p != &id.serial);
        //     out.printer_order.retain(|_, v| v != &id.serial);
        // }

        out
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    #[cfg(feature = "nope")]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("test");
            // self.show_dashboard(ui);

            let src = std::env::var("TEST_IMG").unwrap();
            // debug!("src: {}", src);
            let size = 80.;
            let img = egui::Image::new(&src)
                .fit_to_exact_size(Vec2::new(size, size))
                .max_width(size)
                .max_height(size);
            ui.add(img);
        });
    }

    // #[cfg(feature = "nope")]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // ctx.style_mut(|s| {
        //     s.visuals.dark_mode = true;
        // });
        // ctx.set_visuals(egui::Visuals::dark());
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Main, "Dashboard");
                // ui.selectable_value(&mut self.current_tab, Tab::Graphs, "Graphs");
                // ui.selectable_value(&mut self.current_tab, Tab::Printers, "Printers");
                ui.selectable_value(&mut self.current_tab, Tab::Options, "Options");
            });
        });

        #[cfg(feature = "nope")]
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // egui::widgets::global_dark_light_mode_switch(ui);
                // ui.label("bottom");

                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::initial(50.))
                    .size(egui_extras::Size::initial(20.))
                    .size(egui_extras::Size::initial(20.))
                    .size(egui_extras::Size::initial(20.))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.label("Rows: ");
                        });
                        strip.cell(|ui| {
                            if ui.button(&format!("{}", fill::ARROW_FAT_UP)).clicked() {
                                self.change_rows(false);
                            }
                        });

                        strip.cell(|ui| {
                            ui.label(&format!("{}", self.options.dashboard_size.1));
                        });

                        strip.cell(|ui| {
                            if ui.button(&format!("{}", fill::ARROW_FAT_DOWN)).clicked() {
                                self.change_rows(true);
                            }
                        });
                    });
            });

            // let printer_cfg = &self.config.printers[0];
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.label("bottom");
        });

        match self.current_tab {
            Tab::Main => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    // ui.label("test");
                    self.show_dashboard(ui);
                });
            }
            Tab::Graphs => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.show_graphs(ui);
                });
            }
            Tab::Printers => {
                self.show_printers_config(ctx);
            }
            Tab::Options => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.show_options(ui);
                });
            } // Tab::Debugging => {
              //     // self.show_debugging(ui);
              //     egui::CentralPanel::default().show(ctx, |ui| todo!())
              // }
        }
        //
    }
}

impl App {
    fn show_debugging(&mut self, ui: &mut egui::Ui) {
        ui.label("Debugging");

        egui::Grid::new("debugging_grid").show(ui, |ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut self.debug_host);
            ui.end_row();

            ui.label("Serial:");
            ui.text_edit_singleline(&mut self.debug_serial);
            ui.end_row();

            ui.label("Access Code:");
            ui.text_edit_singleline(&mut self.debug_code);
            ui.end_row();
        });

        if ui.button("Fetch info").clicked() {
            // MARK: TODO
        }
    }
}
