pub mod options;
pub mod plotting;
pub mod printers;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Align, Color32, Layout, Margin, Rounding, Sense, Stroke, Vec2};

use dashmap::DashMap;
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use crate::{
    client::PrinterId,
    config::PrinterConfig,
    icons::*,
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
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
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
        // out.alert_tx = Some(alert_tx);

        out.unplaced_printers = out
            .config
            .printers
            .iter()
            .map(|p| p.serial.clone())
            .collect();
        /// for each printer that isn't in printer_order, queue to add
        for (_, id) in out.printer_order.iter() {
            out.unplaced_printers.retain(|p| p != id);
        }

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
                Tab::Graphs => self.show_graphs(ui),
                Tab::Printers => self.show_printers_config(ui),
                Tab::Options => self.show_options(ui),
                Tab::Debugging => {
                    self.show_debugging(ui);
                }
            }
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.label("bottom");

            // let printer_cfg = &self.config.printers[0];

            #[cfg(feature = "nope")]
            if let Some(mut status) = self.printer_states.get_mut(&printer_cfg.serial) {
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
                            crate::status::PrinterState::Printing,
                            // crate::status::PrinterState::Printing(
                            //     std::time::Instant::now() + Duration::from_secs(92 * 60),
                            // ),
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
            }
        });

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

    /// MARK: show_dashboard
    pub fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        let width = 200.0;
        let height = 350.0;

        // let frame_size = Vec2::new(width, width * (3. / 2.));
        let frame_size = Vec2::new(width, height);
        let item_spacing = 4.;

        egui::Frame::none().show(ui, |ui| {
            let mut max_rect = ui.max_rect();

            max_rect.set_width(max_rect.width() - 2. * item_spacing);

            let num_x = max_rect.width() / (frame_size.x + item_spacing);
            let num_y = max_rect.height() / (frame_size.y + item_spacing);

            // debug!("num_x: {}, num_y: {}", num_x, num_y);

            let width = max_rect.width() / num_x;
            let height = max_rect.height() / num_y;

            let offset_x = Vec2::new(width + item_spacing, 0.);
            let offset_y = Vec2::new(0., height + item_spacing);

            max_rect.set_width(width - item_spacing);

            for y in 0..num_y as usize {
                let mut max_rect_row = max_rect;
                for x in 0..num_x as usize {
                    {
                        let mut ui = ui.child_ui(max_rect_row, *ui.layout());
                        // ui.label(format!("{}x{}", x, y));
                        egui::Frame::group(ui.style()).show(&mut ui, |ui| {
                            let id = if let Some(id) = self.printer_order.get(&(x, y)) {
                                id
                            } else {
                                /// if no printer at this location, try to place one
                                let Some(id) = self.unplaced_printers.pop() else {
                                    return;
                                };

                                self.printer_order.insert((x, y), id.clone());
                                self.printer_order.get(&(x, y)).unwrap()
                            };

                            let Some(printer) =
                                self.config.printers.iter().find(|p| &p.serial == id)
                            else {
                                warn!("Printer not found: {}", id);
                                return;
                            };

                            // let id = &printer_cfg.serial;
                            match self.printer_states.get(id) {
                                Some(printer_state) => self.show_printer(
                                    (x, y),
                                    frame_size,
                                    ui,
                                    printer,
                                    &printer_state,
                                ),
                                None => {
                                    ui.label("Printer not found");
                                }
                            }

                            // let (resp, painter) =
                            //     ui.allocate_painter(ui.available_size(), Sense::hover());

                            // painter.rect_stroke(max_rect_row, Rounding::same(0.5), Stroke::NONE);

                            // ui.allocate_space(ui.available_size());
                            // ui.allocate_space(Vec2::new(ui.available_width(), 0.));
                        });
                    }

                    ui.advance_cursor_after_rect(max_rect_row);
                    max_rect_row = max_rect_row.translate(offset_x);
                }
                max_rect = max_rect.translate(offset_y);
            }
        });

        // egui::Grid::new("printers_grid")
        //     .max_col_width(frame_outer_size.x)
        //     .min_col_width(frame_outer_size.x)
        //     .min_row_height(frame_outer_size.y)
        //     .show(ui, |ui| {
        //         //
        //     });

        #[cfg(feature = "nope")]
        egui_extras::StripBuilder::new(ui)
            .sizes(egui_extras::Size::exact(width), self.config.printers.len())
            .horizontal(|mut strip| {
                for (x, printer_cfg) in self.config.printers.iter().enumerate() {
                    let id = &printer_cfg.serial;
                    let Some(printer) = self.printer_states.get(id) else {
                        return;
                    };

                    strip.cell(|ui| {
                        egui::Frame::group(ui.style())
                            // .outer_margin(egui::Margin::same(100.))
                            .inner_margin(egui::Margin::same(2.))
                            // .fill(Color32::GREEN)
                            .show(ui, |ui| {
                                ui.set_min_size(frame_size);
                                ui.set_max_size(frame_size);
                                self.show_printer(frame_size, ui, printer_cfg, &printer);
                                // ui.allocate_space(Vec2::new(ui.available_width(), frame_size.y));
                                // ui.allocate_space(Vec2::new(ui.available_width(), 0.));
                            });
                    })
                }
            });

        //
    }

    /// MARK: show_printer
    pub fn show_printer(
        &self,
        pos: (usize, usize),
        frame_size: Vec2,
        ui: &mut egui::Ui,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            return;
        };

        ui.dnd_drag_source(
            egui::Id::new(format!("{}_drag_src_{}_{}", printer.serial, pos.0, pos.1)),
            pos,
            |ui| {
                /// printer name
                ui.horizontal(|ui| {
                    paint_icon(ui, 40., &status.state);
                    ui.label(&format!("{} ({})", printer.name, status.state.to_text()));
                });
            },
        );

        ui.add(thumbnail_printer());

        ui.separator();
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);

            ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
            ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
            ui.separator();
            ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
            ui.label(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
            ui.separator();
            ui.add(thumbnail_chamber());
            ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0)));

            ui.allocate_space(Vec2::new(ui.available_width(), 0.));
            ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);
        });
        ui.separator();

        /// fans
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);
            ui.label(&format!("Part: {}%", status.cooling_fan_speed.unwrap_or(0)));
            ui.separator();
            ui.label(&format!("Aux: {}%", status.aux_fan_speed.unwrap_or(0)));
            ui.separator();
            ui.label(&format!("Cham: {}%", status.chamber_fan_speed.unwrap_or(0)));
            ui.allocate_space(Vec2::new(ui.available_width(), 0.));
            ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);
        });
        ui.separator();

        /// current print
        if let PrinterState::Printing = status.state {
            self.show_current_print(frame_size, ui, &status, printer, printer_state);
        } else {
            egui::Grid::new(format!("grid_{}", printer.serial)).show(ui, |ui| {
                ui.label("No print in progress");
                ui.end_row();
                ui.label("--:--");
                ui.end_row();
                ui.label("--:--");
                ui.end_row();
            });
            ui.allocate_space(Vec2::new(ui.available_width(), 0.));
        }

        ui.separator();

        /// controls
        #[cfg(feature = "nope")]
        ui.columns(2, |uis| {
            match &status.state {
                PrinterState::Printing => {
                    if uis[0]
                        .add(egui::Button::image_and_text(icon_pause(), "Pause"))
                        .clicked()
                    {
                        debug!("Pause clicked");
                        // if let Some(tx) = self.alert_tx.as_ref() {
                        //     tx.blocking_send((
                        //         "test alert".to_string(),
                        //         "test message".to_string(),
                        //     ))
                        //     .unwrap();
                        // }
                        // std::thread::spawn(|| {
                        //     crate::alert::alert_message("test alert", "test message");
                        // });
                    }
                    if uis[1]
                        .add(egui::Button::image_and_text(icon_stop(), "Stop"))
                        .clicked()
                    {
                        debug!("Stop clicked");
                    }
                }
                PrinterState::Paused => {
                    if uis[0]
                        .add(egui::Button::image_and_text(icon_resume(), "Resume"))
                        .clicked()
                    {
                        debug!("Resume clicked");
                        // crate::alert::alert_message("test alert", "test message");
                    }
                    if uis[1]
                        .add(egui::Button::image_and_text(icon_stop(), "Stop"))
                        .clicked()
                    {
                        debug!("Stop clicked");
                    }
                }
                PrinterState::Idle => {}
                PrinterState::Error(_) => {}
                PrinterState::Disconnected => {}
                PrinterState::Unknown(s) => {
                    uis[0].label(&format!("Unknown state: {}", &s));
                }
            }
        });

        self.show_controls(frame_size, ui, &status, printer, printer_state);

        ui.separator();
        self.show_ams(frame_size, ui, &status, printer, printer_state);

        //
    }

    fn show_controls(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        status: &PrinterStatus,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        let pause = match &status.state {
            PrinterState::Printing => egui::Button::image_and_text(icon_pause(), "Pause"),
            _ => egui::Button::image_and_text(icon_resume(), "Resume"),
        };
        let stop = egui::Button::image_and_text(icon_stop(), "Stop");

        ui.columns(2, |uis| {
            if uis[0].add(pause).clicked() {
                debug!("Pause clicked");
            }
            if uis[1].add(stop).clicked() {
                debug!("Stop clicked");
            }
        });

        #[cfg(feature = "nope")]
        ui.columns(2, |uis| {
            match &status.state {
                PrinterState::Printing => {
                    if uis[0].add().clicked() {
                        debug!("Pause clicked");
                    }
                    if uis[1]
                        .add(egui::Button::image_and_text(icon_stop(), "Stop"))
                        .clicked()
                    {
                        debug!("Stop clicked");
                    }
                }
                PrinterState::Paused => {
                    if uis[0]
                        .add(egui::Button::image_and_text(icon_resume(), "Resume"))
                        .clicked()
                    {
                        debug!("Resume clicked");
                        // crate::alert::alert_message("test alert", "test message");
                    }
                    if uis[1]
                        .add(egui::Button::image_and_text(icon_stop(), "Stop"))
                        .clicked()
                    {
                        debug!("Stop clicked");
                    }
                }
                PrinterState::Idle => {}
                PrinterState::Error(_) => {}
                PrinterState::Disconnected => {}
                PrinterState::Unknown(s) => {
                    uis[0].label(&format!("Unknown state: {}", &s));
                }
            }
        });

        //
    }

    fn show_ams(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        status: &PrinterStatus,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        let Some(ams) = status.ams.as_ref() else {
            return;
        };

        let Some(unit) = ams.units.get(0) else {
            ui.label("Error getting AMS unit");
            return;
        };

        // let size_x = ui.available_size_before_wrap().x - 4.;
        // let size_x = frame_size.x - 20.;
        // debug!("size_x: {}", size_x);

        let size = 30.;

        ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);
        ui.columns(4, |uis| {
            for i in 0..4 {
                let ui = &mut uis[i];
                // let size = Vec2::splat(size_x / 4.0 - 10.0);
                let size = Vec2::splat(size);
                let (response, painter) = ui.allocate_painter(size, Sense::hover());

                let rect = response.rect;
                let c = rect.center();
                // let r = rect.width() / 2.0 - 1.0;
                let r = size.x / 2.0 - 1.0;

                if let Some(slot) = unit.slots[i].as_ref() {
                    painter.circle_filled(c, r, slot.color);
                } else {
                    painter.circle_stroke(c, r, egui::Stroke::new(1.0, Color32::from_gray(120)));
                }
                // ui.allocate_space(ui.available_size());
            }
        });
        ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);

        //
    }

    /// MARK: show_current_print
    fn show_current_print(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        status: &PrinterStatus,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        if let Some(eta) = status.eta {
            let time = eta.time();
            // let dt = time - chrono::Local::now().naive_local().time();
            let dt = eta - chrono::Local::now();

            // let Some(p) = status.print_percent else {
            //     warn!("no print percent found");
            //     return;
            // };
            // ui.add(
            //     egui::ProgressBar::new(p as f32 / 100.0)
            //         .desired_width(ui.available_width() - 10.)
            //         .text(format!("{}%", p)),
            // );

            egui::Grid::new(format!("grid_{}", printer.serial)).show(ui, |ui| {
                // ui.end_row();

                // ui.label("File:");
                ui.label(
                    status
                        .current_file
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("--"),
                );
                ui.end_row();

                // ui.label("ETA:");
                ui.label(&time.format("%-I:%M %p").to_string());
                ui.end_row();

                // ui.label("Remaining:");
                ui.label(&format!(
                    "-{:02}:{:02}",
                    dt.num_hours(),
                    dt.num_minutes() % 60
                ));
                ui.end_row();
            });

            ui.allocate_space(Vec2::new(ui.available_width(), 0.));
        }

        //
    }
}
