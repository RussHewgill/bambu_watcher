use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{epaint, Color32, Pos2, Rect, Sense, Vec2};
use std::{num, sync::Arc};

use crate::{
    config::PrinterConfig,
    status::{AmsCurrentSlot, AmsStatus, AmsUnit, PrinterState, PrinterType},
};

macro_rules! generate_icon_function {
    ($name:ident, $path:expr, $size:expr) => {
        pub fn $name() -> egui::Image<'static> {
            let size = $size;
            egui::Image::new(egui::include_image!($path))
                .fit_to_exact_size(Vec2::new(size, size))
                .max_width(size)
                .max_height(size)
        }

        paste::paste! {
            pub fn [< $name _with_size >](size: f32) -> egui::Image<'static> {
                egui::Image::new(egui::include_image!($path))
                    .fit_to_exact_size(Vec2::new(size, size))
                    .max_width(size)
                    .max_height(size)
            }
        }
    };
}

generate_icon_function!(icon_empty, "../../assets/icons/empty.svg", 20.);
generate_icon_function!(icon_resume, "../../assets/icons8-play-96.png", 20.);
generate_icon_function!(icon_pause, "../../assets/icons8-pause-squared-100.png", 20.);
generate_icon_function!(icon_stop, "../../assets/icons8-red-square-96.png", 20.);
generate_icon_function!(icon_controls, "../../assets/icons/sliders_poly.svg", 20.);
generate_icon_function!(icon_cloud, "../../assets/icons/cloud-1_poly.svg", 20.);
generate_icon_function!(icon_lan, "../../assets/icons/wifi-100_poly.svg", 20.);
generate_icon_function!(icon_sort_up, "../../assets/icons/sort-up_poly.svg", 20.);
generate_icon_function!(icon_sort_down, "../../assets/icons/sort-down_poly.svg", 20.);
generate_icon_function!(icon_expand, "../../assets/icons/view-expand_poly.svg", 20.);
generate_icon_function!(icon_menu, "../../assets/icons/bars_poly.svg", 20.);

pub fn thumbnail_printer(
    printer: &PrinterConfig,
    printer_type: &PrinterType,
    // size: f32,
    ctx: &egui::Context,
) -> egui::Image<'static> {
    let src = if ctx.style().visuals.dark_mode {
        // egui::include_image!("../../assets/printer_thumbnail_x1.svg")
        match printer_type {
            PrinterType::X1C => egui::include_image!("../../assets/printer_thumbnail_x1.svg"),
            PrinterType::X1E => egui::include_image!("../../assets/printer_thumbnail_x1.svg"),
            PrinterType::P1P => egui::include_image!("../../assets/printer_thumbnail_p1p.svg"),
            PrinterType::P1S => egui::include_image!("../../assets/printer_thumbnail_p1s.svg"),
            PrinterType::A1 => egui::include_image!("../../assets/printer_thumbnail_n2s.svg"),
            PrinterType::A1m => egui::include_image!("../../assets/printer_thumbnail_n1.svg"),
            PrinterType::Unknown => egui::include_image!("../../assets/printer_thumbnail_x1.svg"),
        }
    } else {
        // egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg")
        match printer_type {
            PrinterType::X1C => egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg"),
            PrinterType::X1E => egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg"),
            PrinterType::P1P => egui::include_image!("../../assets/printer_thumbnail_p1p_dark.svg"),
            PrinterType::P1S => egui::include_image!("../../assets/printer_thumbnail_p1s_dark.svg"),
            PrinterType::A1 => egui::include_image!("../../assets/printer_thumbnail_n2s_dark.svg"),
            PrinterType::A1m => egui::include_image!("../../assets/printer_thumbnail_n1_dark.svg"),
            PrinterType::Unknown => {
                egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg")
            }
        }
    };

    egui::Image::new(src)
    // .fit_to_exact_size(Vec2::new(size, size))
    // .max_width(size)
    // .max_height(size)
}

const TEMP_ICON_SIZE: f32 = 20.;

pub fn thumbnail_chamber() -> egui::Image<'static> {
    let size = TEMP_ICON_SIZE;
    let src = egui::include_image!("../../assets/param_chamber_temp.svg");
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_nozzle(active: bool) -> egui::Image<'static> {
    let size = TEMP_ICON_SIZE;
    let src = if active {
        egui::include_image!("../../assets/monitor_nozzle_temp_active.svg")
    } else {
        egui::include_image!("../../assets/monitor_nozzle_temp.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_bed(active: bool) -> egui::Image<'static> {
    let size = TEMP_ICON_SIZE;
    let src = if active {
        egui::include_image!("../../assets/monitor_bed_temp_active.svg")
    } else {
        egui::include_image!("../../assets/monitor_bed_temp.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_fan(on: bool) -> egui::Image<'static> {
    let size = 20.;
    let src = if on {
        egui::include_image!("../../assets/monitor_fan_on.svg")
    } else {
        egui::include_image!("../../assets/monitor_fan_off.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn printer_state_icon(ui: &mut egui::Ui, size: f32, state: &PrinterState) {
    let src = match state {
        // PrinterState::Idle => egui::include_image!("../../assets/icons/check-circle_poly.svg"),
        PrinterState::Idle => egui::include_image!("../../assets/icons/frown_poly.svg"),
        PrinterState::Finished => egui::include_image!("../../assets/icons/frown_poly.svg"),
        PrinterState::Paused => egui::include_image!("../../assets/icons/pause-circle_poly.svg"),
        PrinterState::Printing => egui::include_image!("../../assets/icons/play-circle_poly.svg"),
        PrinterState::Error(_) => {
            egui::include_image!("../../assets/icons/exclamation-triangle_poly.svg")
        }
        PrinterState::Disconnected => {
            egui::include_image!("../../assets/icons/disconnected_poly.svg")
        }
        PrinterState::Unknown(_) => {
            egui::include_image!("../../assets/icons/question-circle_poly.svg")
        }
    };

    #[cfg(feature = "nope")]
    let src = match state {
        PrinterState::Idle => {
            egui::include_image!("../../assets/icons8-hourglass-100.png")
        }
        PrinterState::Idle => {
            egui::include_image!("../../assets/icons8-hourglass-100.png")
        }
        PrinterState::Paused => {
            egui::include_image!("../../assets/icons8-pause-squared-100.png")
        }
        PrinterState::Printing => {
            egui::include_image!("../../assets/icons8-green-circle-96.png")
        }
        PrinterState::Error(_) => {
            egui::include_image!("../../assets/icons8-red-square-96.png")
        }
        PrinterState::Disconnected => {
            egui::include_image!("../../assets/icons8-disconnected-100.png")
        }
        PrinterState::Unknown(s) => {
            ui.label(format!("Unknown: {}", &s));
            return;
        }
    };
    let thumbnail = egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size);
    ui.add(thumbnail);
}

/// Circle for each slot
/// Line going down for active slot
pub fn paint_ams(
    ui: &mut egui::Ui,
    size: f32,
    // size: f32,
    ams: &AmsStatus,
) {
    #[cfg(feature = "nope")]
    ui.vertical(|ui| {
        ui.label(&format!("current tray: {:?}", ams.current_tray));
        ui.label(&format!("tray_now: {:?}", ams.tray_now));
        ui.label(&format!("tray_pre: {:?}", ams.tray_pre));
        ui.label(&format!("tray_tar: {:?}", ams.tray_tar));

        ui.label(&format!("state: {:?}", ams.state));
    });

    let num_units = ams.units.len();

    // debug!("size = {:#?}", size);

    let size = Vec2::new(ui.available_width(), size);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());

    let rect = response.rect;
    let c = rect.center();
    // let r = rect.width() / 2.0 - 1.0;
    // let r = size.x / 2.0 - 1.0;

    // /// 234 x 62
    // // debug!("rect: {:#?}", rect);
    // debug!("rect.width(): {:#?}", rect.width());
    // debug!("rect.height(): {:#?}", rect.height());

    // let mut rect2 = rect;
    // rect2.set_width(rect.width() - 0.);
    // rect2.set_height(rect.height() - 0.0);
    // rect2.set_center(c);
    // painter.rect_stroke(
    //     rect2,
    //     2.,
    //     egui::Stroke::new(3.0, Color32::from_rgba_premultiplied(255, 0, 0, 64)),
    // );

    let p0 = rect.left_top();

    let y = 18.;

    let c = rect.center_top() + Vec2::new(0., y);

    let small_circle_r = 12.;
    let small_spacing = 5.;
    let small_center_spacing = 4.;

    let circle_stroke = 2.;
    let circle_stroke_color = Color32::from_gray(120);
    let y2_height = small_circle_r * 2. + circle_stroke * 2. + 2.;

    if num_units == 0 {
        error!("No units found in ams status");
        return;
    } else if num_units == 1 {
        let unit = &ams.units[&0];

        let edge_padding = rect.width() / 8.0;

        let circle_r = 14.;
        let spacing = (rect.width() - edge_padding * 2.) / 3.0;

        for slot_idx in 0..4 {
            let x = slot_idx as f32 * spacing + edge_padding;
            let c = p0 + Vec2::new(x, y);
            // debug!("c: {:#?}", c);

            match &unit.slots[slot_idx] {
                Some(slot) => {
                    painter.circle(
                        c,
                        circle_r,
                        slot.color,
                        egui::Stroke::new(2., circle_stroke_color),
                    );

                    if let Some(AmsCurrentSlot::Tray { ams_id, tray_id }) = ams.current_tray {
                        if ams_id == 0 && slot_idx as u64 == tray_id {
                            draw_ams_current(&painter, circle_r, circle_stroke, c, slot);
                        }
                    }
                }
                None => {
                    painter.circle_stroke(
                        c,
                        circle_r,
                        egui::Stroke::new(circle_stroke, circle_stroke_color),
                    );
                }
            }
            // let color = unit.slots[slot].as_ref
            // painter.circle_filled(c, r, Color32::RED);
        }
    } else if num_units >= 2 {
        let y1 = c.y;
        let y2 = c.y + y2_height;
        for unit in 0..4 {
            if ams.units.get(&unit).is_none() {
                continue;
            }
            let y = if unit < 2 { y1 } else { y2 };

            let d = if unit % 2 == 0 { -1. } else { 1. };
            for slot_idx in 0..4 {
                let x = c.x
                    + (small_center_spacing + small_circle_r) * d
                    + (small_spacing + small_circle_r * 2.) * slot_idx as f32 * d;

                let c = Pos2::new(x, y);

                match &ams.units[&unit].slots[slot_idx] {
                    Some(slot) => {
                        // painter.circle_filled(c, circle_r, slot.color);
                        painter.circle(
                            c,
                            small_circle_r,
                            slot.color,
                            egui::Stroke::new(circle_stroke, circle_stroke_color),
                        );

                        if let Some(AmsCurrentSlot::Tray { ams_id, tray_id }) = ams.current_tray {
                            if ams_id == unit as u64 && slot_idx as u64 == tray_id {
                                draw_ams_current(&painter, small_circle_r, circle_stroke, c, slot);
                            }
                        }
                    }
                    None => {
                        painter.circle_stroke(
                            c,
                            small_circle_r,
                            egui::Stroke::new(circle_stroke, circle_stroke_color),
                        );
                    }
                }

                //
            }
        }

        let c0 = rect.center_top();
        let c1 = rect.center_top() + Vec2::new(0., small_circle_r * 2. + 2.);
        painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::from_gray(180)));

        if num_units > 2 {
            // let top = y2 - small_circle_r;
            // let c0 = rect.center_top() + Vec2::new(0., top);
            // let c1 = rect.center_top() + Vec2::new(0., top + small_circle_r * 1. + 2.);

            // let top = Pos2::new(c.x, c.y + y2_height);

            // painter.circle_filled(top, 10., Color32::RED);

            let c0 = Pos2::new(c.x, c.y + y2_height - small_circle_r - 2.);
            let c1 = Pos2::new(c.x, c.y + y2_height + small_circle_r + 2.);

            // let c0 = rect.center_top() + Vec2::new(0., top);
            // let c1 = rect.center_top() + Vec2::new(0., top + small_circle_r * 1. + 2.);

            painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::from_gray(180)));
            // painter.line_segment([c0, c1], egui::Stroke::new(1.0, Color32::RED));
        }
    } else {
        debug!("ams.units.len() = {:#?}", ams.units.len());
    }

    //
}

fn draw_ams_current(
    painter: &egui::Painter,
    circle_r: f32,
    circle_stroke: f32,
    c: Pos2,
    slot: &crate::status::AmsSlot,
) {
    // let s = (circle_r + circle_stroke) * 2. + 2.;
    let s = (circle_r + circle_stroke) * 2.;
    let rect2 = Rect::from_center_size(c, Vec2::splat(s));
    painter.rect_stroke(rect2, 3., egui::Stroke::new(circle_stroke, slot.color));
}

#[cfg(feature = "nope")]
pub fn ams_icons_single(
    ui: &mut egui::Ui,
    size: f32,
    wide: bool,
    ams: &AmsUnit,
    current: Option<AmsCurrentSlot>,
) {
    let n = if wide { 4 } else { 8 };
    let size = if wide { size } else { size / 2. };
    ui.columns(n, |uis| {
        for i in 0..4 {
            paint_ams_icons(&mut uis[i], i, size, ams, current);
        }
    });
}

#[cfg(feature = "nope")]
pub fn ams_icons_double(
    ui: &mut egui::Ui,
    size: f32,
    ams0: &AmsUnit,
    ams1: &AmsUnit,
    current: Option<AmsCurrentSlot>,
) {
    ui.columns(8, |uis| {
        for i in 0..4 {
            paint_ams_icons(&mut uis[i], i, size / 2., ams0, current);
        }
        for i in 4..8 {
            paint_ams_icons(&mut uis[i], i - 4, size / 2., ams1, current);
        }
    });
}

#[cfg(feature = "nope")]
fn paint_ams_icons(
    ui: &mut egui::Ui,
    i: usize,
    size: f32,
    ams: &AmsUnit,
    current: Option<AmsCurrentSlot>,
) {
    let size = Vec2::splat(size);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;
    let c = rect.center();
    // let r = rect.width() / 2.0 - 1.0;
    let r = size.x / 2.0 - 1.0;

    if let Some(slot) = ams.slots[i].as_ref() {
        if let Some(current) = current {
            if current.is_slot(ams.id as u64, i as u64) {
                painter.circle_filled(c, r - 3., slot.color);
                let mut rect2 = rect;
                rect2.set_width(rect.width() - 20.);
                rect2.set_height(rect.height() - 1.5);
                rect2.set_center(c);
                painter.rect_stroke(rect2, 3., egui::Stroke::new(3.0, slot.color));
                return;
            }
        }
        painter.circle_filled(c, r - 3., slot.color);
    } else {
        painter.circle_stroke(c, r - 3., egui::Stroke::new(3.0, Color32::from_gray(120)));
    }
}
