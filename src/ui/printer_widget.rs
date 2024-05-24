use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Label, Layout, Response, RichText, Rounding, Vec2};

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::{PrinterConnCmd, PrinterId},
    status::{PrinterState, PrinterStatus},
    ui::{
        icons::*,
        ui_types::{App, GridLocation, Tab},
    },
};

impl App {
    /// Wide layout
    // #[cfg(feature = "nope")]
    pub fn show_printer(
        &self,
        pos: (usize, usize),
        frame_size: Vec2,
        ui: &mut egui::Ui,
        printer: &PrinterConfig,
    ) -> Response {
        /// checked at call site
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };

        /// Name, state, and controls button
        /// Can't be in strip or response can't get passed up
        let resp = ui
            .horizontal(|ui| {
                let selected = self
                    .selected_printer_controls
                    .as_ref()
                    .map(|s| s == &printer.serial)
                    .unwrap_or(false);

                /// cloud button
                #[cfg(feature = "nope")]
                {
                    let cloud = printer.cloud.load(std::sync::atomic::Ordering::Relaxed);
                    let icon = if cloud {
                        super::icons::icon_cloud()
                    } else {
                        super::icons::icon_lan()
                    };

                    if ui.add(egui::Button::image(icon)).clicked() {
                        self.cmd_tx
                            .as_ref()
                            .unwrap()
                            .send(PrinterConnCmd::SetPrinterCloud(
                                printer.serial.clone(),
                                !cloud,
                            ))
                            .unwrap();
                    }
                }

                #[cfg(feature = "nope")]
                if ui
                    .add(egui::Button::image(super::icons::icon_controls()).selected(selected))
                    .clicked()
                {
                    if selected {
                        self.selected_printer_controls = None;
                    } else {
                        self.selected_printer_controls = Some(printer.serial.clone());
                    }
                }

                ui.dnd_drag_source(
                    egui::Id::new(format!("{}_drag_src_{}_{}", printer.serial, pos.0, pos.1)),
                    GridLocation {
                        col: pos.0,
                        row: pos.1,
                    },
                    |ui| {
                        paint_icon(ui, 40., &status.state);
                        ui.add(
                            egui::Label::new(&format!(
                                "{} ({})",
                                printer.name,
                                status.state.to_text()
                            ))
                            .truncate(true),
                        );
                    },
                )
                .response
            })
            .response;

        let layout = Layout::left_to_right(egui::Align::Center)
            .with_cross_justify(true)
            .with_main_justify(true)
            .with_cross_align(egui::Align::Center);

        let text_size_title = 14.;
        let text_size_eta = 12.;

        let thumbnail_width = frame_size.x - 12.;
        let thumbnail_height = thumbnail_width * 0.5625;

        ui.spacing_mut().item_spacing.x = 1.;
        egui_extras::StripBuilder::new(ui)
            .cell_layout(layout)
            // thumbnail
            // .size(egui_extras::Size::exact(frame_size.x * 0.5625 + 3.))
            .size(egui_extras::Size::exact(thumbnail_height + 6.))
            // temperatures
            .size(egui_extras::Size::exact(26.))
            // Title
            .size(egui_extras::Size::exact(text_size_title + 4.))
            // progress bar
            .size(egui_extras::Size::exact(26.))
            // ETA
            .size(egui_extras::Size::exact(text_size_eta + 2.))
            // AMS
            .size(egui_extras::Size::exact(60. + 2.))
            // .size(egui_extras::Size::initial(10.))
            .vertical(|mut strip| {
                /// thumbnail/webcam
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::GREEN, "");
                    let layout = Layout::left_to_right(egui::Align::Center)
                        // .with_cross_justify(true)
                        .with_main_justify(true)
                        .with_cross_align(egui::Align::Center);

                    ui.with_layout(layout, |ui| {
                        // debug!("width = {}, height = {}", thumbnail_width, thumbnail_height);

                        if let Some(entry) = self.printer_textures.get(&printer.serial) {
                            // debug!("entry.size_vec2() = {:?}", entry.size_vec2());
                            // let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                            let size = Vec2::new(thumbnail_width, thumbnail_height);
                            let img = egui::Image::from_texture((entry.id(), size))
                                .fit_to_exact_size(size)
                                .max_size(size)
                                .rounding(Rounding::same(4.));
                            // .max_width(width);
                            // .max_height(size);
                            ui.add(img);
                        } else if let Some(url) = status.current_task_thumbnail_url.as_ref() {
                            let img = egui::Image::new(url)
                                .bg_fill(if ui.visuals().dark_mode {
                                    Color32::from_gray(128)
                                } else {
                                    Color32::from_gray(210)
                                })
                                // .fit_to_exact_size(Vec2::new(size, size))
                                .max_width(thumbnail_width)
                                // .max_height(size)
                                .rounding(Rounding::same(4.));
                            ui.add(img);
                        } else if let Some(t) = status.printer_type {
                            ui.add(
                                thumbnail_printer(&printer, &t, ui.ctx())
                                    // .max_width(width)
                                    .fit_to_exact_size(Vec2::new(thumbnail_width, thumbnail_height))
                                    .rounding(Rounding::same(4.)),
                            );
                        }
                    });
                });

                /// temperatures
                strip.strip(|mut builder| {
                    let font_size = 12.;

                    // let layout = Layout::left_to_right(egui::Align::Center)
                    //     .with_cross_justify(true)
                    //     .with_main_justify(true)
                    //     .with_cross_align(egui::Align::Center);

                    builder
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::remainder())
                        .cell_layout(layout)
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    // ui.ctx().debug_painter().debug_rect(
                                    //     ui.max_rect(),
                                    //     Color32::RED,
                                    //     "",
                                    // );
                                    ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                                    ui.add(
                                        Label::new(
                                            // RichText::new(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)))
                                            RichText::new(format!(
                                                "{:.1}°C / {}",
                                                status.temp_nozzle.unwrap_or(0.),
                                                status.temp_tgt_nozzle.unwrap_or(0.0) as i64
                                            ))
                                            .size(font_size),
                                        )
                                        .truncate(true),
                                    );
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                                    ui.add(
                                        Label::new(
                                            RichText::new(format!(
                                                "{:.1}°C / {}",
                                                status.temp_bed.unwrap_or(0.),
                                                status.temp_tgt_bed.unwrap_or(0.0) as i64
                                            ))
                                            .size(font_size),
                                        )
                                        .truncate(true),
                                    );
                                });
                            });
                            strip.cell(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(thumbnail_chamber());
                                    ui.label(
                                        RichText::new(format!(
                                            "{}°C",
                                            status.temp_chamber.unwrap_or(0.) as i64
                                        ))
                                        .size(font_size),
                                    );
                                });
                            });
                        });
                });

                /// Title
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::GREEN, "");
                    let layout = Layout::left_to_right(egui::Align::Min)
                        .with_cross_justify(true)
                        .with_main_justify(true)
                        .with_cross_align(egui::Align::Min);

                    ui.with_layout(layout, |ui| {
                        ui.add(
                            Label::new(
                                RichText::new(&format!(
                                    "{}",
                                    status
                                        .current_file
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("--"),
                                ))
                                .size(text_size_title),
                            )
                            .truncate(true),
                        );
                    });
                });

                /// progress bar
                strip.cell(|ui| {
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");
                    let p = status.print_percent.unwrap_or(0);
                    ui.add(
                        egui::ProgressBar::new(p as f32 / 100.0)
                            .desired_width(ui.available_width() - 0.)
                            .text(format!("{}%", p)),
                    );
                });

                /// ETA
                /// TODO: layers?
                strip.strip(|mut builder| {
                    let Some(eta) = status.eta else {
                        return;
                    };

                    let time = eta.time();
                    // let dt = time - chrono::Local::now().naive_local().time();
                    let dt = if eta < chrono::Local::now() {
                        chrono::TimeDelta::zero()
                    } else {
                        eta - chrono::Local::now()
                    };

                    builder
                        .size(egui_extras::Size::relative(0.4))
                        .size(egui_extras::Size::remainder())
                        .size(egui_extras::Size::relative(0.4))
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                // ui.ctx().debug_painter().debug_rect(
                                //     ui.max_rect(),
                                //     Color32::GREEN,
                                //     "",
                                // );
                                ui.add(Label::new(
                                    RichText::new(&time.format("%-I:%M %p").to_string())
                                        .size(text_size_eta),
                                ));
                            });
                            strip.cell(|ui| {});
                            strip.cell(|ui| {
                                ui.add(Label::new(
                                    RichText::new(&format!(
                                        "-{:02}:{:02}",
                                        dt.num_hours(),
                                        dt.num_minutes() % 60
                                    ))
                                    .size(text_size_eta),
                                ));
                            });
                        });
                });

                /// AMS
                strip.cell(|ui| {
                    self.show_ams(frame_size, ui, printer);
                    // ui.ctx()
                    //     .debug_painter()
                    //     .debug_rect(ui.max_rect(), Color32::RED, "");
                });

                //
            });

        resp
    }

    /// Wide layout
    #[cfg(feature = "nope")]
    pub fn show_printer(
        &mut self,
        pos: (usize, usize),
        frame_size: Vec2,
        ui: &mut egui::Ui,
        printer: &PrinterConfig,
    ) -> Response {
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };
        /// checked at call site
        let printer_state = self.printer_states.get(&printer.serial).unwrap();

        /// Name, state, and controls button
        let resp = ui
            .horizontal(|ui| {
                let selected = self
                    .selected_printer_controls
                    .as_ref()
                    .map(|s| s == &printer.serial)
                    .unwrap_or(false);

                /// cloud button
                #[cfg(feature = "nope")]
                {
                    let cloud = printer.cloud.load(std::sync::atomic::Ordering::Relaxed);
                    let icon = if cloud {
                        super::icons::icon_cloud()
                    } else {
                        super::icons::icon_lan()
                    };

                    if ui.add(egui::Button::image(icon)).clicked() {
                        self.cmd_tx
                            .as_ref()
                            .unwrap()
                            .send(PrinterConnCmd::SetPrinterCloud(
                                printer.serial.clone(),
                                !cloud,
                            ))
                            .unwrap();
                    }
                }

                #[cfg(feature = "nope")]
                if ui
                    .add(egui::Button::image(super::icons::icon_controls()).selected(selected))
                    .clicked()
                {
                    if selected {
                        self.selected_printer_controls = None;
                    } else {
                        self.selected_printer_controls = Some(printer.serial.clone());
                    }
                }

                ui.dnd_drag_source(
                    egui::Id::new(format!("{}_drag_src_{}_{}", printer.serial, pos.0, pos.1)),
                    GridLocation {
                        col: pos.0,
                        row: pos.1,
                    },
                    |ui| {
                        paint_icon(ui, 40., &status.state);
                        ui.add(
                            egui::Label::new(&format!(
                                "{} ({})",
                                printer.name,
                                status.state.to_text()
                            ))
                            .truncate(true),
                        );
                    },
                )
                .response
            })
            .response;

        /// TODO: center the thumbnail
        // #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            let size = frame_size.x - 4.;
            if let Some(entry) = self.printer_textures.get(&printer.serial) {
                let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                    .rounding(Rounding::same(4.))
                    .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .max_height(size);
                ui.add(img);
            } else if let Some(url) = printer_state.current_task_thumbnail_url.as_ref() {
                let img = egui::Image::new(url)
                    .bg_fill(if ui.visuals().dark_mode {
                        Color32::from_gray(128)
                    } else {
                        Color32::from_gray(210)
                    })
                    .rounding(Rounding::same(4.))
                    .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .max_height(size);
                ui.add(img);
            } else if let Some(t) = printer_state.printer_type {
                ui.add(
                    thumbnail_printer(&printer, &t, size, ui.ctx()).rounding(Rounding::same(4.)),
                );
            }
        });

        // let mut rect = ui.cursor();
        // /// 16:9 aspect ratio
        // rect.set_height(frame_size.x * 0.5625);

        /// thumbnail / webcam
        #[cfg(feature = "nope")]
        ui.allocate_ui_at_rect(rect, |ui| {
            egui::Frame::none().show(ui, |ui| {
                let size = frame_size.x - 12.;
                if let Some(entry) = self.printer_textures.get(&printer.serial) {
                    let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                    // .bg_fill(if ui.visuals().dark_mode {
                    //     Color32::from_gray(128)
                    // } else {
                    //     Color32::from_gray(210)
                    // })
                    .rounding(Rounding::same(4.))
                    // .shrink_to_fit()
                    // .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .maintain_aspect_ratio(true)
                    // .max_height(size);
                    ;
                    ui.add(img);
                } else if let Some(url) = printer_state.current_task_thumbnail_url.as_ref() {
                    // debug!("url = {}", url);
                    let img = egui::Image::new(url)
                        .bg_fill(if ui.visuals().dark_mode {
                            Color32::from_gray(128)
                        } else {
                            Color32::from_gray(210)
                        })
                        .rounding(Rounding::same(4.))
                        // .shrink_to_fit()
                        .fit_to_exact_size(Vec2::new(size, size))
                        .max_width(size)
                        .max_height(size);
                    ui.add(img);
                } else if let Some(t) = printer_state.printer_type {
                    ui.add(
                        thumbnail_printer(&printer, &t, size, ui.ctx())
                            .rounding(Rounding::same(4.)),
                    );
                }

                ui.allocate_space(ui.available_size());
            });
        });

        /// Temperatures
        #[cfg(feature = "nope")]
        {
            ui.separator();

            let mut rect = ui.cursor();
            rect.set_height(40.);
            rect.set_width(frame_size.x - 12.);

            ui.allocate_ui_at_rect(rect, |ui| {
                ui.columns(3, |uis| {
                    let font_size = 12.;
                    uis[0].horizontal(|ui| {
                        ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                        ui.add(
                            Label::new(
                                // RichText::new(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)))
                                RichText::new(format!(
                                    "{:.1}°C / {}",
                                    status.temp_nozzle.unwrap_or(0.),
                                    status.temp_tgt_nozzle.unwrap_or(0.0) as i64
                                ))
                                .size(font_size),
                            )
                            .truncate(true),
                        );
                    });
                    uis[1].horizontal(|ui| {
                        ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                        ui.add(
                            Label::new(
                                RichText::new(format!(
                                    "{:.1}°C / {}",
                                    status.temp_bed.unwrap_or(0.),
                                    status.temp_tgt_bed.unwrap_or(0.0) as i64
                                ))
                                .size(font_size),
                            )
                            .truncate(true),
                        );
                    });
                    uis[2].horizontal(|ui| {
                        ui.add(thumbnail_chamber());
                        ui.label(
                            RichText::new(format!(
                                "{}°C",
                                status.temp_chamber.unwrap_or(0.) as i64
                            ))
                            .size(font_size),
                        );
                    });
                });
            });
        }

        resp
    }

    #[cfg(feature = "nope")]
    /// Tall layout
    pub fn show_printer(
        &mut self,
        pos: (usize, usize),
        frame_size: Vec2,
        ui: &mut egui::Ui,
        printer: &PrinterConfig,
    ) -> Response {
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };
        /// checked at call site
        let printer_state = self.printer_states.get(&printer.serial).unwrap();

        /// Name, state, and controls button
        let resp = ui
            .horizontal(|ui| {
                let selected = self
                    .selected_printer_controls
                    .as_ref()
                    .map(|s| s == &printer.serial)
                    .unwrap_or(false);

                /// cloud button
                #[cfg(feature = "nope")]
                {
                    let cloud = printer.cloud.load(std::sync::atomic::Ordering::Relaxed);
                    let icon = if cloud {
                        super::icons::icon_cloud()
                    } else {
                        super::icons::icon_lan()
                    };

                    if ui.add(egui::Button::image(icon)).clicked() {
                        self.cmd_tx
                            .as_ref()
                            .unwrap()
                            .send(PrinterConnCmd::SetPrinterCloud(
                                printer.serial.clone(),
                                !cloud,
                            ))
                            .unwrap();
                    }
                }

                #[cfg(feature = "nope")]
                if ui
                    .add(egui::Button::image(super::icons::icon_controls()).selected(selected))
                    .clicked()
                {
                    if selected {
                        self.selected_printer_controls = None;
                    } else {
                        self.selected_printer_controls = Some(printer.serial.clone());
                    }
                }

                ui.dnd_drag_source(
                    egui::Id::new(format!("{}_drag_src_{}_{}", printer.serial, pos.0, pos.1)),
                    GridLocation {
                        col: pos.0,
                        row: pos.1,
                    },
                    |ui| {
                        paint_icon(ui, 40., &status.state);
                        ui.add(
                            egui::Label::new(&format!(
                                "{} ({})",
                                printer.name,
                                status.state.to_text()
                            ))
                            .truncate(true),
                        );
                    },
                )
                .response
            })
            .response;

        let mut rect = ui.cursor();
        /// 16:9 aspect ratio
        rect.set_height(frame_size.x * 0.5625);

        /// thumbnail / webcam
        ui.allocate_ui_at_rect(rect, |ui| {
            egui::Frame::none().show(ui, |ui| {
                let size = frame_size.x - 12.;
                if let Some(entry) = self.printer_textures.get(&printer.serial) {
                    let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                    // .bg_fill(if ui.visuals().dark_mode {
                    //     Color32::from_gray(128)
                    // } else {
                    //     Color32::from_gray(210)
                    // })
                    .rounding(Rounding::same(4.))
                    // .shrink_to_fit()
                    // .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .maintain_aspect_ratio(true)
                    // .max_height(size);
                    ;
                    ui.add(img);
                } else if let Some(url) = printer_state.current_task_thumbnail_url.as_ref() {
                    // debug!("url = {}", url);
                    let img = egui::Image::new(url)
                        .bg_fill(if ui.visuals().dark_mode {
                            Color32::from_gray(128)
                        } else {
                            Color32::from_gray(210)
                        })
                        .rounding(Rounding::same(4.))
                        // .shrink_to_fit()
                        .fit_to_exact_size(Vec2::new(size, size))
                        .max_width(size)
                        .max_height(size);
                    ui.add(img);
                } else if let Some(t) = printer_state.printer_type {
                    ui.add(
                        thumbnail_printer(&printer, &t, size, ui.ctx())
                            .rounding(Rounding::same(4.)),
                    );
                }

                ui.allocate_space(ui.available_size());
            });
        });

        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            let size = 80. - 4.;

            // let d = include_bytes!("../../test.jpg");
            // let data = std::fs::read("test.jpg").unwrap();

            // let image = image::load_from_memory(&data).unwrap();
            // let img_size = [image.width() as _, image.height() as _];
            // let image_buffer = image.to_rgba8();
            // let pixels = image_buffer.as_flat_samples();
            // let img = egui::ColorImage::from_rgba_unmultiplied(img_size, pixels.as_slice());

            // let entry = self
            //     .printer_textures
            //     .entry(printer.serial.clone())
            //     .or_insert_with(|| {
            //         let handle = ui.ctx().load_texture(
            //             format!("{}_tex", printer.serial.clone()),
            //             img,
            //             Default::default(),
            //         );
            //         handle
            //     });

            if let Some(entry) = self.printer_textures.get(&printer.serial) {
                let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                    // .bg_fill(if ui.visuals().dark_mode {
                    //     Color32::from_gray(128)
                    // } else {
                    //     Color32::from_gray(210)
                    // })
                    .rounding(Rounding::same(4.))
                    // .shrink_to_fit()
                    // .fit_to_exact_size(Vec2::new(size, size))
                    // .max_width(size)
                    .maintain_aspect_ratio(true)
                    .max_height(size);
                ui.add(img);
            } else if let Some(url) = printer_state.current_task_thumbnail_url.as_ref() {
                // debug!("url = {}", url);
                let img = egui::Image::new(url)
                    .bg_fill(if ui.visuals().dark_mode {
                        Color32::from_gray(128)
                    } else {
                        Color32::from_gray(210)
                    })
                    .rounding(Rounding::same(4.))
                    // .shrink_to_fit()
                    .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .max_height(size);
                ui.add(img);
            } else if let Some(t) = printer_state.printer_type {
                ui.add(
                    thumbnail_printer(&printer, &t, size, ui.ctx()).rounding(Rounding::same(4.)),
                );
            }

            /// temperatures
            #[cfg(feature = "nope")]
            ui.vertical(|ui| {
                // egui::Frame::none().fill(Color32::RED).show(ui, |ui| {
                ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);

                ui.horizontal(|ui| {
                    ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                    ui.label(format!(
                        "{:.1}°C / {}",
                        status.temp_nozzle.unwrap_or(0.),
                        status.temp_tgt_nozzle.unwrap_or(0.) as i64,
                    ));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                    ui.label(format!(
                        "{:.1}°C / {}",
                        status.temp_bed.unwrap_or(0.),
                        status.temp_tgt_bed.unwrap_or(0.) as i64
                    ));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(thumbnail_chamber());
                    ui.label(format!("{}°C", status.temp_chamber.unwrap_or(0.) as i64));
                });

                ui.allocate_space(Vec2::new(ui.available_width(), 0.));
                ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);
                // });
            });
        });
        ui.separator();

        let mut rect = ui.cursor();
        rect.set_height(40.);
        rect.set_width(frame_size.x - 12.);

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.columns(3, |uis| {
                let font_size = 10.;
                uis[0].horizontal(|ui| {
                    ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                    ui.label(
                        RichText::new(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.)))
                            .size(font_size),
                    );
                });
                uis[1].horizontal(|ui| {
                    ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                    ui.label(
                        RichText::new(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)))
                            .size(font_size),
                    );
                });
                uis[2].horizontal(|ui| {
                    ui.add(thumbnail_chamber());
                    ui.label(
                        RichText::new(format!("{}°C", status.temp_chamber.unwrap_or(0.) as i64))
                            .size(font_size),
                    );
                });
            });
        });

        /// temperatures
        #[cfg(feature = "nope")]
        ui.allocate_ui_at_rect(rect, |ui| {
            egui::Frame::none().show(ui, |ui| {
                let size = (frame_size.x / 3.) - 4.;
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(size))
                    .size(egui_extras::Size::exact(size))
                    .size(egui_extras::Size::exact(size))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                            ui.small(format!("{:.1}°C", status.temp_nozzle.unwrap_or(0.),));
                        });
                        strip.cell(|ui| {
                            ui.add(thumbnail_bed(status.temp_tgt_bed.is_some()));
                            ui.small(format!("{:.1}°C", status.temp_bed.unwrap_or(0.)));
                        });
                        strip.cell(|ui| {
                            ui.add(thumbnail_chamber());
                            ui.small(format!("{}°C", status.temp_chamber.unwrap_or(0.) as i64));
                        });
                    });
            });
        });

        /// temperatures
        #[cfg(feature = "nope")]
        ui.columns(3, |uis| {
            uis[0].horizontal(|ui| {
                ui.add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
                ui.small(format!(
                    "{:.1}°C / {}",
                    status.temp_nozzle.unwrap_or(0.),
                    status.temp_tgt_nozzle.unwrap_or(0.) as i64,
                ));
            });
            uis[1].horizontal(|ui| {
                //
            });
            uis[2].horizontal(|ui| {
                ui.add(thumbnail_chamber());
                ui.small(format!("{}°C", status.temp_chamber.unwrap_or(0.) as i64));
            });
        });

        #[cfg(feature = "nope")]
        /// temperatures
        ui.columns(3, |uis| {
            uis[0].add(thumbnail_nozzle(status.temp_tgt_nozzle.is_some()));
            uis[0].label(format!(
                "{:.1}°C / {}",
                status.temp_nozzle.unwrap_or(0.),
                status.temp_tgt_nozzle.unwrap_or(0.) as i64,
            ));

            uis[1].add(thumbnail_bed(status.temp_tgt_bed.is_some()));
            uis[1].vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(format!(
                    "{:.1}°C",
                    status.temp_bed.unwrap_or(0.)
                ))));
                ui.add(egui::Label::new(RichText::new(format!(
                    "{:.1}",
                    status.temp_tgt_bed.unwrap_or(0.)
                ))));
            });
            // uis[1].label(format!(
            //     "{:.1}°C",
            //     status.temp_bed.unwrap_or(0.),
            //     status.temp_tgt_bed.unwrap_or(0.) as i64
            // ));

            uis[2].add(thumbnail_chamber());
            uis[2].label(format!("{}°C", status.temp_chamber.unwrap_or(0.) as i64));
        });

        /// fans
        #[cfg(feature = "nope")]
        ui.columns(3, |uis| {
            uis[0].label(RichText::new("Part:").text_style(egui::TextStyle::Small));
            uis[0].label(&format!("{: >4}%", status.cooling_fan_speed.unwrap_or(0)));

            uis[1].label(RichText::new("Aux:").text_style(egui::TextStyle::Small));
            uis[1].label(&format!("{: >4}%", status.aux_fan_speed.unwrap_or(0)));

            uis[2].label(RichText::new("Cham:").text_style(egui::TextStyle::Small));
            uis[2].label(&format!("{: >4}%", status.chamber_fan_speed.unwrap_or(0)));
        });

        /// fans
        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);

            ui.vertical(|ui| {
                ui.label(RichText::new("Part:").text_style(egui::TextStyle::Small));
                ui.label(&format!("{: >4}%", status.cooling_fan_speed.unwrap_or(0)))
            });

            ui.separator();
            ui.vertical(|ui| {
                ui.label(RichText::new("Aux:").text_style(egui::TextStyle::Small));
                ui.label(&format!("{: >4}%", status.aux_fan_speed.unwrap_or(0)))
            });

            ui.separator();
            ui.vertical(|ui| {
                ui.label(RichText::new("Cham:").text_style(egui::TextStyle::Small));
                ui.label(&format!("{: >4}%", status.chamber_fan_speed.unwrap_or(0)))
            });

            // ui.label(
            //     RichText::new(format!("Part: {}%", status.cooling_fan_speed.unwrap_or(0)))
            //         .text_style(egui::TextStyle::Small),
            // );
            // // ui.label(&format!("Part: {}%", status.cooling_fan_speed.unwrap_or(0)));
            // ui.separator();
            // ui.label(&format!("Aux: {}%", status.aux_fan_speed.unwrap_or(0)));
            // ui.separator();
            // ui.label(&format!("Cham: {}%", status.chamber_fan_speed.unwrap_or(0)));
            // ui.allocate_space(Vec2::new(ui.available_width(), 0.));
            ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);
        });
        ui.separator();

        /// current print
        self.show_current_print(frame_size, ui, &status, &printer, &printer_state);
        #[cfg(feature = "nope")]
        if let PrinterState::Printing = status.state {
            self.show_current_print(frame_size, ui, &status, printer.clone(), &printer_state);
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
        self.show_controls(frame_size, ui, &status, printer, &printer_state);

        ui.separator();
        // self.show_ams(frame_size, ui, &status, printer, &printer_state);
        drop(status);
        drop(printer_state);
        self.show_ams(
            frame_size, ui, // &status,
            printer,
            // &mut self.selected_ams,
        );

        ui.separator();

        //
        resp
    }
}

impl App {
    /// MARK: control
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

    /// MARK: ams
    fn show_ams(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        // status: &PrinterStatus,
        printer: &PrinterConfig,
        // selected_ams: &mut HashMap<PrinterId, usize>,
        // printer_state: &PrinterStatus,
    ) {
        let Some(status) = self.printer_states.get(&printer.serial) else {
            warn!("Printer not found: {}", printer.serial);
            panic!();
        };

        let Some(ams) = status.ams.as_ref() else {
            return;
        };

        // let size_x = ui.available_size_before_wrap().x - 4.;
        // let size_x = frame_size.x - 20.;
        // debug!("size_x: {}", size_x);

        let size = 30.;

        let num_ams = ams.units.len();
        // let mut ams_id = self.selected_ams.entry(printer.serial.clone()).or_default();

        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            if ui.button("-").clicked() {
                if *ams_id == 0 {
                    *ams_id = num_ams - 1;
                } else {
                    *ams_id -= 1;
                }
            }
            ui.label(&format!("{}", ams_id));
            if ui.button("+").clicked() {
                if *ams_id >= num_ams - 1 {
                    *ams_id = 0;
                } else {
                    *ams_id += 1;
                }
            }
        });

        // egui_extras::StripBuilder::new(ui)
        //     .size(egui_extras::Size::exact(30.))
        //     .size(egui_extras::Size::exact(30.))
        //     .size(egui_extras::Size::exact(30.))
        //     .horizontal(|mut strip| {
        //         strip.cell(|ui| {
        //             if ui.button("+").clicked() {
        //                 if *ams_id >= num_ams - 1 {
        //                     *ams_id = 0;
        //                 } else {
        //                     *ams_id += 1;
        //                 }
        //             }
        //         });
        //         strip.cell(|ui| {
        //             ui.label(&format!("{}", ams_id));
        //         });
        //         strip.cell(|ui| {
        //             if ui.button("-").clicked() {
        //                 if *ams_id == 0 {
        //                     *ams_id = num_ams - 1;
        //                 } else {
        //                     *ams_id -= 1;
        //                 }
        //             }
        //         });
        //     });

        // let Some(unit) = ams.units.get(0) else {
        //     ui.label("No AMS Connected");
        //     return;
        // };

        if num_ams == 0 {
            ui.label("No AMS Connected");
            return;
        } else if num_ams == 1 {
            ams_icons_single(ui, size, true, ams.units.get(&0).unwrap())
        } else if num_ams == 2 {
            ams_icons_double(
                ui,
                size,
                ams.units.get(&0).unwrap(),
                ams.units.get(&1).unwrap(),
            );
        } else if num_ams == 3 {
            ams_icons_double(
                ui,
                size,
                ams.units.get(&0).unwrap(),
                ams.units.get(&1).unwrap(),
            );
            ams_icons_single(ui, size, false, ams.units.get(&2).unwrap())
            // ams_icons(ui, false, ams.units.get(0).unwrap());
            // ams_icons(ui, false, ams.units.get(1).unwrap());
            // ams_icons(ui, false, ams.units.get(2).unwrap());
        } else if num_ams == 4 {
            ams_icons_double(
                ui,
                size,
                ams.units.get(&0).unwrap(),
                ams.units.get(&1).unwrap(),
            );
            ams_icons_double(
                ui,
                size,
                ams.units.get(&2).unwrap(),
                ams.units.get(&3).unwrap(),
            );
        } else {
            ui.label(format!("Too many AMS units: {}", num_ams));
            warn!("Too many AMS units: {}", num_ams);
            return;
        }

        ui.style_mut().spacing.item_spacing = Vec2::new(1., 1.);
        #[cfg(feature = "nope")]
        ui.horizontal(|ui| {
            let n = 4 * ams.units.len();

            let size = size / ams.units.len() as f32;

            ui.columns(n, |uis| {
                // let mut ams_id = self.selected_ams.entry(printer.serial.clone()).or_default();

                #[cfg(feature = "nope")]
                {
                    let ui = &mut uis[0];

                    ui.add(
                        // egui::Slider::new(ams_id, 0..=ams.units.len())
                        //     .show_value(false)
                        //     .vertical(),
                        egui::DragValue::new(ams_id)
                            // .speed(1.0)
                            .clamp_range(0..=ams.units.len() - 1),
                    );

                    // if ui.button("+").clicked() {
                    //     *ams_id += 1;
                    // }
                    // ui.label(&format!("{}", ams_id));
                    // if ui.button("-").clicked() {
                    //     *ams_id -= 1;
                    // }
                }

                for ams_id in 0..ams.units.len().min(2) {
                    for i in 0..4 {
                        let ui = &mut uis[ams_id * 4 + i];
                        // let size = Vec2::splat(size_x / 4.0 - 10.0);
                        let size = Vec2::splat(size);
                        let (response, painter) = ui.allocate_painter(size, Sense::hover());

                        let rect = response.rect;
                        let c = rect.center();
                        // let r = rect.width() / 2.0 - 1.0;
                        let r = size.x / 2.0 - 1.0;

                        let Some(unit) = ams.units.get(ams_id) else {
                            error!("AMS unit not found");
                            panic!("AMS unit not found");
                        };

                        if let Some(slot) = unit.slots[i].as_ref() {
                            painter.circle_filled(c, r, slot.color);
                        } else {
                            painter.circle_stroke(
                                c,
                                r,
                                egui::Stroke::new(1.0, Color32::from_gray(120)),
                            );
                        }
                        // ui.allocate_space(ui.available_size());
                    }
                }
            });
            ui.style_mut().spacing.item_spacing = Vec2::new(8., 3.);
        });

        //
    }

    /// MARK: show_current_print
    #[cfg(feature = "nope")]
    fn show_current_print(
        &self,
        frame_size: Vec2,
        ui: &mut egui::Ui,
        status: &PrinterStatus,
        printer: &PrinterConfig,
        printer_state: &PrinterStatus,
    ) {
        let Some(eta) = status.eta else {
            return;
        };

        let time = eta.time();
        // let dt = time - chrono::Local::now().naive_local().time();
        let dt = if eta < chrono::Local::now() {
            chrono::TimeDelta::zero()
        } else {
            eta - chrono::Local::now()
        };

        let Some(p) = status.print_percent else {
            warn!("no print percent found");
            return;
        };
        ui.add(
            egui::ProgressBar::new(p as f32 / 100.0)
                .desired_width(ui.available_width() - 0.)
                .text(format!("{}%", p)),
        );

        ui.add(
            egui::Label::new(
                status
                    .current_file
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("--"),
            )
            .truncate(true),
        );

        ui.horizontal(|ui| {
            ui.label(&time.format("%-I:%M %p").to_string());
            ui.separator();
            ui.label(&format!(
                "-{:02}:{:02}",
                dt.num_hours(),
                dt.num_minutes() % 60
            ));
        });

        #[cfg(feature = "nope")]
        egui::Grid::new(format!("grid_{}", printer.serial))
            .min_col_width(ui.available_width() - 4.)
            .show(ui, |ui| {
                // ui.label("File:");
                ui.add(
                    egui::Label::new(
                        status
                            .current_file
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("--"),
                    )
                    .truncate(true),
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

                // ui.allocate_space(Vec2::new(ui.available_width(), 0.));
            });

        ui.allocate_space(Vec2::new(ui.available_width(), 0.));

        //
    }
}
