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
    status::{bambu::PrinterStatus, PrinterState},
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

        let width = 250.0;
        let height = 340.0;

        // let frame_size = Vec2::new(width, width * (3. / 2.));
        let frame_size = Vec2::new(width, height);
        let item_spacing = 4.;

        #[cfg(feature = "nope")]
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::containers::ScrollArea::both()
                .auto_shrink(false)
                // .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    let (cols, rows) = self.options.dashboard_size;
                    ui.spacing_mut().item_spacing = [0.0; 2].into();
                    egui_extras::StripBuilder::new(ui)
                        .sizes(egui_extras::Size::relative((rows as f32).recip()), rows)
                        .vertical(|mut strip| {
                            for r in 0..rows {
                                strip.cell(|ui| {
                                    egui_extras::StripBuilder::new(ui)
                                        .sizes(
                                            egui_extras::Size::relative((cols as f32).recip()),
                                            cols,
                                        )
                                        .horizontal(|mut strip| {
                                            for c in 0..cols {
                                                strip.cell(|ui| {
                                                    let i = r * cols + c;
                                                    ui.ctx().debug_painter().debug_rect(
                                                        ui.max_rect(),
                                                        Color32::GREEN,
                                                        format!("{i} ({r}, {c})"),
                                                    );
                                                })
                                            }
                                        });
                                });
                            }
                        });
                });
        });

        // #[cfg(feature = "nope")]
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

                            let pos = GridLocation { col: x, row: y };
                            let (id, color) = self.get_printer_id_color(pos);

                            // #[cfg(feature = "nope")]
                            ui.allocate_ui_at_rect(max_rect_row, |ui| {
                                if let Some(color) = color {
                                    ui.visuals_mut().widgets.inactive.bg_stroke =
                                        Stroke::new(4., color);
                                    ui.visuals_mut().widgets.active.bg_stroke =
                                        Stroke::new(4., color);
                                }
                                let frame = egui::Frame::group(ui.style())
                                    .inner_margin(4.)
                                    .outer_margin(4.)
                                    // .stroke(Stroke::new(5., color))
                                    // .stroke(Stroke::new(50., Color32::RED))
                                    // .fill(color)
                                    .rounding(6.);

                                let (_, dropped_payload) =
                                    ui.dnd_drop_zone::<GridLocation, ()>(frame, |ui| {
                                        let Some(id) = id else {
                                            ui.label("Empty");
                                            ui.allocate_space(ui.available_size());
                                            return;
                                        };

                                        // let id = if let Some(id) = self.printer_order.get(&pos) {
                                        //     id.clone()
                                        // } else {
                                        //     /// if no printer at this location, try to place one
                                        //     let Some(id) = self.unplaced_printers.pop() else {
                                        //         ui.label("Empty");
                                        //         ui.allocate_space(ui.available_size());
                                        //         return;
                                        //     };

                                        //     self.printer_order.insert(pos, id.clone());
                                        //     // self.printer_order.get(&pos).unwrap()
                                        //     id
                                        // };

                                        let Some(printer) = self.config.get_printer(&id) else {
                                            warn!("Printer not found: {}", id);
                                            return;
                                        };

                                        if self.printer_states.contains_key(&id) {
                                            let resp = if let Ok(printer) = printer.try_read() {
                                                // egui::Frame::none()
                                                //     // .fill(color)
                                                //     // .fill(color)
                                                //     .inner_margin(Margin::same(6.))
                                                //     // .inner_margin(0.)
                                                //     .outer_margin(0.)
                                                //     .rounding(6.)
                                                //     .show(ui, |ui| {
                                                //         let resp = self.show_printer(
                                                //             (x, y),
                                                //             frame_size,
                                                //             ui,
                                                //             // id,
                                                //             &printer,
                                                //             // &printer_state,
                                                //         );
                                                //         ui.allocate_space(ui.available_size());
                                                //         resp
                                                //     })

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
                                        // ui.allocate_space(ui.available_size());
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

    fn get_printer_id_color(&mut self, pos: GridLocation) -> (Option<PrinterId>, Option<Color32>) {
        let id = if let Some(id) = self.printer_order.get(&pos) {
            id.clone()
        } else {
            /// if no printer at this location, try to place one
            let Some(id) = self.unplaced_printers.pop() else {
                return (None, None);
            };

            self.printer_order.insert(pos, id.clone());
            id
        };

        let Some(printer) = self.config.get_printer(&id) else {
            warn!("Printer not found: {}", id);
            return (None, None);
        };

        let color = if let Some(status) = self.printer_states.get(&id) {
            match &status.state {
                PrinterState::Paused => Color32::from_rgb(173, 125, 90),
                PrinterState::Printing => Color32::from_rgb(121, 173, 116),
                PrinterState::Error(_) => Color32::from_rgb(173, 125, 90),
                _ => Color32::from_gray(127),
                // _ => Color32::GREEN,
            }
        } else {
            // debug!("no state");
            // Color32::from_gray(127)
            Color32::RED
        };

        (Some(id), Some(color))
    }

    pub fn show_stream(&mut self, ctx: &egui::Context, id: PrinterId) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(entry) = self.printer_textures.get(&id) else {
                return;
            };
            if !entry.enabled {
                self.selected_stream = None;
            }
            let entry = entry.handle.clone();

            let size = ui.available_size();

            // let size = Vec2::new(thumbnail_width, thumbnail_height);
            let img = egui::Image::from_texture((entry.id(), entry.size_vec2()))
                // .fit_to_exact_size(size)
                .max_size(size)
                .maintain_aspect_ratio(true)
                .rounding(Rounding::same(4.))
                .sense(Sense::click());
            if ui.add(img).clicked() {
                self.selected_stream = None;
            }
        });
    }

    /// MARK: show_control_panel
    pub fn show_control_panel(&mut self, ui: &mut egui::Ui, printer: Arc<PrinterConfig>) {
        // ui.label(&format!("Printer: {:?}", self.selected_printer_controls));
        //
    }
}
