use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{Align, Color32, Layout, Margin, Sense, Vec2};
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
    pub fn show_dashboard(&self, ui: &mut egui::Ui) {
        let width = 200.0;

        let frame_size = Vec2::new(width, width * (3. / 2.));
        // let frame_margin = ui.style().spacing.item_spacing.x;

        // let available_width = ui.available_width();

        // let num_x = (available_width / (frame_size.x + frame_margin)).floor() as usize;
        // let num_x = num_x.min(self.config.printers.len());
        // let num_y = (self.printer_states.len() as f32 / num_x as f32).ceil() as usize;

        // ui.horizontal(|ui| {
        //     for printer in self.config.printers.iter() {
        //         // ui.label(&printer.serial);
        //     }
        // });

        // debug!("num_x: {}, num_y: {}", num_x, num_y);

        #[cfg(feature = "nope")]
        egui_extras::TableBuilder::new(ui)
            .columns(
                egui_extras::Column::exact(width),
                self.config.printers.len(),
            )
            .auto_shrink(true)
            .body(|mut body| {
                // body.ui_mut().style_mut().spacing.item_spacing.x = 50.;
                body.row(frame_size.y, |mut row| {
                    for printer_cfg in self.config.printers.iter() {
                        let id = &printer_cfg.serial;
                        let Some(printer) = self.printer_states.get(id) else {
                            row.col(|ui| {});
                            continue;
                        };

                        row.col(|ui| {
                            egui::Frame::group(ui.style())
                                // .outer_margin(egui::Margin::same(10.))
                                // .inner_margin(egui::Margin::same(10.))
                                .show(ui, |ui| {
                                    self.show_printer(frame_size, ui, printer_cfg, &printer);
                                    // ui.allocate_space(Vec2::new(
                                    //     ui.available_width(),
                                    //     frame_size.y,
                                    // ));
                                });
                        });
                    }
                });
            });

        // #[cfg(feature = "nope")]
        egui_extras::StripBuilder::new(ui)
            .sizes(egui_extras::Size::exact(width), self.config.printers.len())
            .horizontal(|mut strip| {
                for printer_cfg in self.config.printers.iter() {
                    let id = &printer_cfg.serial;
                    let Some(printer) = self.printer_states.get(id) else {
                        return;
                    };

                    strip.cell(|ui| {
                        // // ui.set_max_size(frame_size);
                        // egui::Frame::none()
                        //     // .inner_margin(egui::Margin::same(10.))
                        //     .fill(Color32::RED)
                        //     .show(ui, |ui| {
                        //     });
                        egui::Frame::group(ui.style())
                            // .outer_margin(egui::Margin::same(100.))
                            // .inner_margin(egui::Margin::same(10.))
                            // .fill(Color32::GREEN)
                            .show(ui, |ui| {
                                self.show_printer(frame_size, ui, printer_cfg, &printer);
                                ui.allocate_space(Vec2::new(ui.available_width(), frame_size.y));
                            });
                    })
                }
            });

        // ui.allocate_ui_with_layout(frame_size, Layout::top_down(Align::LEFT), |ui| {
        //     ui.group(|ui| {
        //         // ui.set_min_size(frame_size);
        //         // ui.set_max_size(frame_size);
        //         self.show_printer(frame_size, ui, printer_cfg, &printer);
        //         // ui.allocate_space(ui.available_size());
        //     });
        // });

        #[cfg(feature = "nope")]
        for y in 0..num_y {
            ui.columns(num_x, |uis| {
                for x in 0..num_x {
                    let idx = y * num_x + x;
                    if idx >= self.config.printers.len() {
                        debug!("x: {}, y: {}", x, y);
                        warn!("idx out of bounds: {}", idx);
                        break;
                    }

                    let printer_cfg = &self.config.printers[idx];
                    let id = &printer_cfg.serial;
                    let Some(printer) = self.printer_states.get(id) else {
                        continue;
                    };
                    uis[x].group(|ui| {
                        self.show_printer(frame_size, ui, printer_cfg, &printer);
                    });
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

        /// printer name
        ui.horizontal(|ui| {
            paint_icon(ui, 40., &status.state);
            ui.label(&format!("{} ({})", printer.name, status.state.to_text()));
        });

        ui.add(thumbnail_printer());

        // egui_extras::StripBuilder::new(ui)
        //     .sizes(egui_extras::Size::exact(frame_size.x / 3. - 5.), 3)
        //     .horizontal(|mut strip| {
        //         strip.cell(|ui| {
        //             ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
        //             ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
        //             // ui.separator();
        //         });
        //         strip.cell(|ui| {
        //             ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
        //             ui.label(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
        //             //         ui.separator();
        //         });

        //         strip.cell(|ui| {
        //             ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0)));
        //         });
        //     });

        egui::Frame::group(ui.style())
            // .outer_margin(egui::Margin::same(100.))
            // .inner_margin(egui::Margin::same(10.))
            // .fill(Color32::GREEN)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let item_spacing = ui.spacing().item_spacing.x;
                    /// [[20.0 148.0] - [218.0 580.0]]
                    let mut max_rect = ui.max_rect();
                    max_rect.set_width(frame_size.x - 2.);
                    // debug!("max_rect: {:?}", max_rect);
                    max_rect.set_width(max_rect.width() - 2. * item_spacing); // adjust it so it does not include spacing
                    let width = max_rect.width() / 3.; // get width for the widgets

                    // debug!("width: {}", width);

                    let offset = Vec2::new(width + item_spacing, 0.); // compute the offset for subsequent rects
                    max_rect.set_width(width); // and set the width of rect

                    ui.allocate_ui_at_rect(max_rect, |ui| {
                        // ui.horizontal(|ui| {
                        ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                        ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
                        // });
                    });

                    ui.separator();
                    max_rect = max_rect.translate(offset);

                    ui.allocate_ui_at_rect(max_rect, |ui| {
                        // ui.horizontal(|ui| {
                        ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                        ui.label(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
                        // });
                    });

                    ui.separator();
                    max_rect = max_rect.translate(offset);

                    ui.allocate_ui_at_rect(max_rect, |ui| {
                        // ui.horizontal(|ui| {
                        ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0)));
                        // });
                    });

                    ui.allocate_space(Vec2::new(ui.available_size().x, 0.));
                });
            });

        #[cfg(feature = "nope")]
        {
            /// temperatures
            let layout = Layout::left_to_right(Align::Center)
                .with_main_justify(true)
                // .with_cross_justify(true)
                // .with_main_justify(true)
                ;

            /// size = 195, 30
            let size = Vec2::new(frame_size.x - 5., 30.);
            // debug!("size = {:?}", size);
            ui.allocate_ui_with_layout(size, layout, |ui| {
                ui.set_min_size(size);
                ui.set_max_size(size);
                egui::Frame::group(ui.style())
                    // .outer_margin(egui::Margin::same(100.))
                    // .inner_margin(egui::Margin::same(10.))
                    // .fill(Color32::GREEN)
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        ui.set_max_size(size);
                        ui.style_mut().spacing.item_spacing.x = 1.;
                        ui.style_mut().spacing.item_spacing.y = 1.;

                        {
                            ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                            ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
                            ui.separator();

                            ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                            ui.label(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
                            ui.separator();

                            ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0)));
                        }

                        // ui.columns(3, |uis| {
                        //     uis[0].add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                        //     uis[0].label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
                        //     //         ui.separator();
                        //     uis[1].add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                        //     uis[1].label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
                        //     uis[2].label(format!("{}°C", status.temp_chamber.unwrap_or(0)));
                        // });
                    });
            });
        }

        // /// temperatures
        //     let layout = Layout::left_to_right(Align::Center)
        //         .with_main_justify(true)
        //         // .with_main_justify(true)
        //         ;
        // let size = Vec2::new(frame_size.x - 5., 30.);
        // // ui.set_max_size(size);
        // // debug!("size = {:?}", size);
        // ui.allocate_ui_with_layout(size, layout, |ui| {
        //     ui.set_max_size(size);
        //     ui.group(|ui| {
        //         ui.style_mut().spacing.item_spacing.x = 1.;
        //         ui.style_mut().spacing.item_spacing.y = 1.;

        //         ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
        //         ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
        //         ui.separator();

        //         ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
        //         ui.label(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
        //         ui.separator();

        //         ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0)));
        //     });
        //     #[cfg(feature = "nope")]
        //     ui.horizontal(|ui| {});
        // });

        /// current print
        ui.group(|ui| {
            if let PrinterState::Printing = status.state {
                self.show_current_print(frame_size, ui, &status, printer, printer_state);
            } else {
                ui.label("No print in progress");
            }
        });

        /// controls
        ui.group(|ui| {
            //
        });

        //
    }

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
            let dt = time - chrono::Local::now().naive_local().time();

            egui::Grid::new(format!("grid_{}", printer.serial)).show(ui, |ui| {
                ui.label("File:");
                ui.label(
                    status
                        .current_file
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("--"),
                );
                ui.end_row();

                ui.label("ETA:");
                ui.label(&time.format("%-I:%M %p").to_string());
                ui.end_row();

                ui.label("Remaining:");
                ui.label(&format!(
                    "{:02}:{:02}",
                    dt.num_hours(),
                    dt.num_minutes() % 60
                ));
                ui.end_row();
            });
        }

        //
    }

    #[cfg(feature = "nope")]
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
                    paint_icon(ui, 40., &status.state);
                    ui.label(&format!("{} ({})", printer.name, status.state.to_text()));
                });

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.add(thumbnail);

                        ui.columns(3, |uis| {
                            uis[0].group(|ui| {
                                ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                                ui.label(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)));
                                //
                            });
                        });
                    });

                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            if let PrinterState::Printing = status.state {
                                if let Some(eta) = status.eta {
                                    // use std::time::{SystemTime, UNIX_EPOCH};
                                    // let now = SystemTime::now();
                                    // let duration_since_epoch =
                                    //     now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                                    // let datetime: chrono::DateTime<chrono::Utc> = UNIX_EPOCH.into();
                                    // let datetime = datetime
                                    //     + chrono::Duration::from_std(duration_since_epoch)
                                    //         .expect("Failed to convert duration");

                                    let time = eta.time();
                                    let dt = time - chrono::Local::now().naive_local().time();

                                    egui::Grid::new(format!("grid_{}", printer.serial)).show(
                                        ui,
                                        |ui| {
                                            ui.label("File:");
                                            ui.label(
                                                status
                                                    .current_file
                                                    .as_ref()
                                                    .map(|s| s.as_str())
                                                    .unwrap_or("--"),
                                            );
                                            ui.end_row();

                                            ui.label("ETA:");
                                            ui.label(&time.format("%-I:%M %p").to_string());
                                            ui.end_row();

                                            ui.label("Remaining:");
                                            ui.label(&format!(
                                                "{:02}:{:02}",
                                                dt.num_hours(),
                                                dt.num_minutes() % 60
                                            ));
                                            ui.end_row();
                                        },
                                    );
                                }
                                // ui.label("ETA: {:02}:{:02}", , 23);
                            } else {
                                ui.label("ETA: --:--");
                            }
                        });
                        ui.allocate_space(ui.available_size());
                    });
                });
            });
    }
}

fn thumbnail_printer() -> egui::Image<'static> {
    let size = 80.;
    egui::Image::new(egui::include_image!("../assets/printer_thumbnail_x1.svg"))
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

fn thumbnail_nozzle(active: bool) -> egui::Image<'static> {
    let size = 20.;
    let src = if active {
        egui::include_image!("../assets/monitor_nozzle_temp_active.svg")
    } else {
        egui::include_image!("../assets/monitor_nozzle_temp.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

fn thumbnail_bed(active: bool) -> egui::Image<'static> {
    let size = 20.;
    let src = if active {
        egui::include_image!("../assets/monitor_bed_temp_active.svg")
    } else {
        egui::include_image!("../assets/monitor_bed_temp.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn paint_icon(ui: &mut egui::Ui, size: f32, state: &PrinterState) {
    let src = match state {
        PrinterState::Idle => {
            egui::include_image!("../assets/icons8-hourglass-100.png")
        }
        PrinterState::Paused => {
            egui::include_image!("../assets/icons8-pause-squared-100.png")
        }
        PrinterState::Printing => {
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
