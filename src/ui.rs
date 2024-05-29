pub mod dashboard;
pub mod icons;
pub mod options;
pub mod plotting;
pub mod printer_widget;
pub mod printers;
pub mod project_view;
pub mod ui_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Align, Color32, Layout, Margin, Response, Rounding, Sense, Stroke, Vec2};
use egui_phosphor::fill;

use dashmap::DashMap;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use crate::{
    cloud::streaming::{StreamCmd, WebcamTexture},
    config::{ConfigArc, PrinterConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg, PrinterId},
    status::{PrinterState, PrinterStatus},
    ui::{
        icons::*,
        ui_types::{App, GridLocation, Tab},
    },
};

// pub use self::ui_types::;

pub mod error_message {
    use anyhow::{anyhow, bail, ensure, Context, Result};
    use egui::{Label, RichText, TextStyle};
    use tracing::{debug, error, info, trace, warn};

    pub fn run_error_app(error: String) -> eframe::Result<()> {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                // .with_icon(icon)
                .with_resizable(false)
                .with_max_inner_size([300., 200.])
                .with_inner_size([300.0, 200.0]),
            ..Default::default()
        };

        eframe::run_native(
            "Bambu Watcher Error",
            native_options,
            Box::new(move |cc| Box::new(ErrorApp { error })),
        )
    }

    pub struct ErrorApp {
        pub error: String,
    }

    impl eframe::App for ErrorApp {
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(Label::new(
                        RichText::new("Error:").text_style(TextStyle::Heading),
                    ));
                    ui.label(&self.error);
                });
            });
        }
    }
}

/// new
impl App {
    pub fn new(
        // tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        // tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        config: ConfigArc,
        cc: &eframe::CreationContext<'_>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        msg_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>,
        stream_cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
        printer_textures: Arc<DashMap<PrinterId, WebcamTexture>>,
        graphs: plotting::Graphs,
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
        out.msg_rx = Some(msg_rx);
        out.stream_cmd_tx = Some(stream_cmd_tx);

        out.graphs = Some(graphs);

        out.unplaced_printers = out.config.printer_ids();
        /// for each printer that isn't in printer_order, queue to add
        for (_, id) in out.printer_order.iter() {
            out.unplaced_printers.retain(|p| p != id);
        }

        out.printer_textures = printer_textures;

        /// remove printers that were previously placed but are no longer in the config
        {
            let current_printers = out
                .config
                .printer_ids()
                .into_iter()
                // .map(|c| c.serial.clone())
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

/// MARK: update TODO
/// - wifi signal
/// - controls on side
/// - light
/// - temp targets
/// - layer progress
/// - AMS humidity
impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.read_channels();

        // ctx.style_mut(|s| {
        //     s.visuals.dark_mode = true;
        // });
        // ctx.set_visuals(egui::Visuals::dark());
        if cfg!(debug_assertions) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Dashboard, "Dashboard");
                // ui.selectable_value(&mut self.current_tab, Tab::Graphs, "Graphs");
                // ui.selectable_value(&mut self.current_tab, Tab::Printers, "Printers");
                // ui.selectable_value(&mut self.current_tab, Tab::Projects, "Projects");
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
            Tab::Dashboard => {
                if let Some(id) = self.selected_stream.as_ref() {
                    self.show_stream(ctx, id.clone());
                } else {
                    self.show_dashboard(ctx);
                }
            }
            Tab::Graphs => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.show_graphs(ui);
                });
            }
            Tab::Projects => {
                self.show_project_view(ctx);
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

/// read channels
impl App {
    fn read_channels(&mut self) {
        let rx = self.msg_rx.as_mut().unwrap();

        let msg = match rx.try_recv() {
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => return,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                error!("Disconnected from printer connection manager");
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            PrinterConnMsg::StatusReport(_, _) => {}
            PrinterConnMsg::LoggedIn => {}
            PrinterConnMsg::SyncedProjects(projects) => {
                self.projects = projects;
            }
            _ => {
                warn!("unhandled message: {:?}", msg);
            }
        }
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
