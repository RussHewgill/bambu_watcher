use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{
    Align, Color32, Layout, Margin, Rect, Response, RichText, Rounding, Sense, Stroke, Vec2,
};

use dashmap::DashMap;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

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
    /// MARK: show_dashboard
    pub fn show_dashboard(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "nope")]
        if self.selected_printer_controls.is_some() {
            egui::panel::SidePanel::right("printer_controls").show(ctx, |ui| {
                let Some(id) = self.selected_printer_controls.as_ref().cloned() else {
                    error!("No printer selected");
                    return;
                };
                let Some(printer) = self.config.get_printer(&id) else {
                    warn!("Printer not found: {}", id);
                    return;
                };
                self.show_control_panel(ui, printer.clone());
            });
        }

        let width = 200.0;
        let height = 350.0;

        /// actually ends up 200 x 330?
        // let frame_size = Vec2::new(width, width * (3. / 2.));
        let frame_size = Vec2::new(width, height);
        let item_spacing = 4.;

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::containers::ScrollArea::both()
                .auto_shrink(false)
                // .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let mut max_rect = ui.max_rect();

                    max_rect.set_width(width);
                    max_rect.set_height(height);

                    let offset_x = Vec2::new(width + item_spacing, 0.);
                    let offset_y = Vec2::new(0., height + item_spacing);

                    let mut from = None;
                    let mut to = None;

                    /// XXX Why is 3 the magic number?
                    let n = 3.;
                    let (w, h) = (width - item_spacing * n, height - item_spacing * n);

                    // ui.visuals_mut().widgets.inactive.bg_fill = Color32::RED;
                    // ui.visuals_mut().widgets.active.bg_fill = Color32::RED;

                    for y in 0..self.options.dashboard_size.1 {
                        let mut max_rect_row = max_rect;
                        for x in 0..self.options.dashboard_size.0 {
                            let i = x + y * self.options.dashboard_size.0;

                            // #[cfg(feature = "nope")]
                            ui.allocate_ui_at_rect(max_rect_row, |ui| {
                                let frame = egui::Frame::group(ui.style());

                                let (_, dropped_payload) =
                                    ui.dnd_drop_zone::<GridLocation, ()>(frame, |ui| {
                                        let pos = GridLocation { col: x, row: y };
                                        let id = if let Some(id) = self.printer_order.get(&pos) {
                                            id
                                        } else {
                                            /// if no printer at this location, try to place one
                                            let Some(id) = self.unplaced_printers.pop() else {
                                                ui.label("Empty");
                                                ui.allocate_space(ui.available_size());
                                                return;
                                            };

                                            self.printer_order.insert(pos, id);
                                            self.printer_order.get(&pos).unwrap()
                                        };

                                        let Some(printer) = self.config.get_printer(&id) else {
                                            warn!("Printer not found: {}", id);
                                            return;
                                        };

                                        if self.printer_states.contains_key(id) {
                                            let resp = if let Ok(printer) = printer.try_read() {
                                                self.show_printer(
                                                    (x, y),
                                                    frame_size,
                                                    ui,
                                                    // id,
                                                    &printer,
                                                    // &printer_state,
                                                )
                                            } else {
                                                warn!("Printer not found");
                                                panic!();
                                            };
                                        } else {
                                            ui.label("Printer not found");
                                            // ui.allocate_space(Vec2::new(w, h));
                                            ui.allocate_space(ui.available_size());
                                            return;
                                        }

                                        #[cfg(feature = "nope")]
                                        match self.printer_states.get(id) {
                                            Some(_printer_state) => {
                                                let resp = self.show_printer(
                                                    (x, y),
                                                    frame_size,
                                                    ui,
                                                    // id,
                                                    printer.clone(),
                                                    // &printer_state,
                                                );

                                                // /// TODO: Preview
                                                // if let (Some(pointer), Some(hovered_payload)) = (
                                                //     ui.input(|i| i.pointer.interact_pos()),
                                                //     resp.response.dnd_hover_payload::<GridLocation>(),
                                                // ) {
                                                //     // debug!("dropped from {:?}", hovered_payload);
                                                //     //
                                                // }
                                            }
                                            None => {
                                                ui.label("Printer not found");
                                                // ui.allocate_space(Vec2::new(w, h));
                                                ui.allocate_space(ui.available_size());
                                                return;
                                            }
                                        }

                                        // ui.label("Frame");
                                        ui.allocate_space(ui.available_size());
                                    });

                                if let Some(dragged_payload) = dropped_payload {
                                    from = Some(dragged_payload);
                                    to = Some(GridLocation { col: x, row: y });
                                }
                            });

                            max_rect_row = max_rect_row.translate(offset_x);
                        }
                        max_rect = max_rect.translate(offset_y);
                    }

                    if let (Some(from), Some(to)) = (from, to) {
                        self.move_printer(&from, &to);
                    }
                });
        });

        //
    }

    /// MARK: show_control_panel
    pub fn show_control_panel(&mut self, ui: &mut egui::Ui, printer: Arc<PrinterConfig>) {
        // ui.label(&format!("Printer: {:?}", self.selected_printer_controls));
        //
    }

    // #[cfg(feature = "nope")]
    /// MARK: show_printer
    pub fn show_printer(
        &mut self,
        pos: (usize, usize),
        frame_size: Vec2,
        ui: &mut egui::Ui,
        // id: &PrinterId,
        printer: &PrinterConfig,
        // printer_state: &PrinterStatus,
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

        ui.horizontal(|ui| {
            let size = 80. - 4.;
            if let Some(url) = printer_state.current_task_thumbnail_url.as_ref() {
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

        /// fans
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
        &mut self,
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
