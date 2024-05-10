use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{Color32, Margin, Sense, Vec2};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use crate::{
    client::PrinterId,
    config::PrinterConfig,
    status::{PrinterState, PrinterStatus},
    ui_types::{App, Tab},
};

impl App {
    pub fn new(
        tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        // tray_icon: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        config: crate::config::Config,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        let mut out = if let Some(storage) = cc.storage {
            let mut out: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            out
        } else {
            Self::default()
        };

        out.tray = tray_icon;
        out.printer_states = printer_states;
        out.config = config;
        out
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Main, "Dashboard");
                ui.selectable_value(&mut self.current_tab, Tab::Options, "Options");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.label("test");

            match self.current_tab {
                Tab::Main => self.show_dashboard(ui),
                // Tab::Options => self.show_options(ui),
                Tab::Options => unimplemented!(),
            }
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.label("bottom");

            let printer_cfg = &self.config.printers[0];

            let mut status = self.printer_states.get_mut(&printer_cfg.serial).unwrap();

            egui::ComboBox::from_label("Set State")
                .selected_text(status.state.to_char())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut status.state,
                        crate::status::PrinterState::Disconnected,
                        "Disconnected",
                    );
                    ui.selectable_value(
                        &mut status.state,
                        crate::status::PrinterState::Idle,
                        "Idle",
                    );
                    ui.selectable_value(
                        &mut status.state,
                        crate::status::PrinterState::Printing(
                            std::time::Instant::now() + Duration::from_secs(92 * 60),
                        ),
                        "Printing",
                    );
                    ui.selectable_value(
                        &mut status.state,
                        crate::status::PrinterState::Paused,
                        "Paused",
                    );
                    ui.selectable_value(
                        &mut status.state,
                        crate::status::PrinterState::Error("Error".to_string()),
                        "Error",
                    );
                });
        });

        //
    }
}

impl App {
    pub fn show_dashboard(&self, ui: &mut egui::Ui) {
        let frame_size = Vec2::new(200., 100.);
        let frame_margin = ui.style().spacing.item_spacing.x;

        let available_width = ui.available_width();

        let num_x = (available_width / (frame_size.x + frame_margin)).floor() as usize;
        let num_x = num_x.min(self.config.printers.len());
        let num_y = (self.printer_states.len() as f32 / num_x as f32).ceil() as usize;

        // ui.horizontal(|ui| {
        //     for printer in self.config.printers.iter() {
        //         // ui.label(&printer.serial);
        //     }
        // });

        for y in 0..num_y {
            ui.columns(num_x, |uis| {
                for x in 0..num_x {
                    let idx = y * num_x + x;
                    if idx >= self.printer_states.len() {
                        warn!("idx out of bounds: {}", idx);
                        break;
                    }

                    let printer_cfg = &self.config.printers[idx];
                    let id = &printer_cfg.serial;
                    let printer = self.printer_states.get(id).unwrap();
                    self.show_printer(frame_size, &mut uis[x], printer_cfg, &printer);
                }
            });
        }
    }

    pub fn show_printer(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            return;
        };

        // let s: &PrinterStatus = &status;

        // ui.label(printer_id);
        egui::Frame::group(ui.style())
            // .stroke(egui::Stroke::new(1.0, egui::Color32::RED))
            // .shadow(shadow)
            // .fill(egui::Color32::from_gray(230))
            // .inner_margin(Margin::same(1.0))
            .show(ui, |ui| {
                ui.set_min_size(frame_size);

                ui.horizontal(|ui| {
                    // ui.label(status.state.to_char());
                    paint_icon(ui, 40., &status.state);
                    ui.label(&format!("{} ({})", printer.name, status.state.to_text()));
                });

                ui.horizontal(|ui| {
                    let thumbnail = egui::Image::new(egui::include_image!(
                        "../assets/printer_thumbnail_x1.svg"
                    ))
                    .fit_to_exact_size(Vec2::new(80., 80.))
                    .max_width(80.)
                    .max_height(80.);
                    ui.add(thumbnail);

                    ui.group(|ui| {
                        if let PrinterState::Printing(end) = status.state {
                            // ui.label("ETA: {:02}:{:02}", , 23);
                        } else {
                            ui.label("ETA: --:--");
                        }
                        ui.allocate_space(ui.available_size());
                    });
                });
            });
    }
}

pub fn paint_icon(ui: &mut egui::Ui, size: f32, state: &PrinterState) {
    let src = match state {
        PrinterState::Idle => {
            egui::include_image!("../assets/icons8-hourglass-100.png")
        }
        PrinterState::Paused => {
            egui::include_image!("../assets/icons8-pause-squared-100.png")
        }
        PrinterState::Printing(_) => {
            egui::include_image!("../assets/icons8-green-circle-96.png")
        }
        PrinterState::Error(_) => {
            egui::include_image!("../assets/icons8-red-square-96.png")
        }
        PrinterState::Disconnected => {
            egui::include_image!("../assets/icons8-disconnected-100.png")
        }
    };
    let thumbnail = egui::Image::new(src).max_width(size).max_height(size);
    ui.add(thumbnail);
}
