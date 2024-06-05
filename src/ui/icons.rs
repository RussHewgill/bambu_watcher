use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{epaint, Color32, Pos2, Sense, Vec2};
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
    let r = size.x / 2.0 - 1.0;

    // /// 234 x 62
    // // debug!("rect: {:#?}", rect);
    // debug!("rect.width(): {:#?}", rect.width());
    // debug!("rect.height(): {:#?}", rect.height());

    // let mut rect2 = rect;
    // rect2.set_width(rect.width() - 0.);
    // rect2.set_height(rect.height() - 0.0);
    // rect2.set_center(c);
    // painter.rect_stroke(rect2, 2., egui::Stroke::new(3.0, Color32::RED));

    let p0 = rect.left_top();

    let y = 16.;

    if num_units == 0 {
        error!("No units found in ams status");
        return;
    } else if num_units == 1 {
        let unit = &ams.units[&0];

        let edge_padding = rect.width() / 8.0;

        let circle_r = 14.;
        let spacing = (rect.width() - edge_padding * 2.) / 3.0;

        for slot in 0..4 {
            let x = slot as f32 * spacing + edge_padding;
            let c = p0 + Vec2::new(x, y);
            // debug!("c: {:#?}", c);

            match &unit.slots[slot] {
                Some(slot) => {
                    painter.circle_filled(c, circle_r, slot.color);
                }
                None => {
                    painter.circle_stroke(
                        c,
                        circle_r,
                        egui::Stroke::new(3.0, Color32::from_gray(120)),
                    );
                }
            }
            // let color = unit.slots[slot].as_ref
            // painter.circle_filled(c, r, Color32::RED);
        }
    } else if num_units == 2 {
        // /// 9 spaces
        // /// 2 edge spaces
        // /// 1 space between units
        // /// 6 spaces between slots
        // let edge_padding = rect.width() / 9.0;
        // let spacing = (rect.width() - edge_padding * 3.) / 7.0;
        // let group_padding = edge_padding;

        // debug!("edge_padding: {:#?}", edge_padding);
        // debug!("spacing: {:#?}", spacing);
        // debug!("group_padding: {:#?}", group_padding);

        // // let circle_r = 14.;
        // let circle_r = spacing / 2. - 2.;

        // debug!("circle_r: {:#?}", circle_r);

        // let edge_padding = 26.;
        // let spacing = 22.2857;
        // let group_padding = 26.;
        // let circle_r = 9.14;

        // let edge_padding = 20.;
        // let spacing = 22.;
        // let group_padding = 26.;
        // let circle_r = 10.;

        #[cfg(feature = "nope")]
        for unit in 0..2 {
            for slot in 0..4 {
                // let x = (slot as f32 * (4 * unit) as f32) * spacing + edge_padding;
                // debug!("x: {:#?}", x);

                /// edge_padding
                /// 4 slots
                /// edge_padding
                /// 4 slots
                /// edge_padding
                let x = ((unit * 4) as f32 + slot as f32) * spacing
                    + edge_padding
                    + (unit as f32 * group_padding);

                // let x = edge_padding
                //     + (unit as f32 * spacing)
                //     // + (unit as f32 * spacing)
                //     ;

                let c = p0 + Vec2::new(x, y);
                // debug!("c: {:#?}", c);

                match &ams.units[&unit].slots[slot] {
                    Some(slot) => {
                        // painter.circle_filled(c, circle_r, slot.color);
                        painter.circle(
                            c,
                            circle_r,
                            slot.color,
                            egui::Stroke::new(1., Color32::from_gray(120)),
                        );
                    }
                    None => {
                        painter.circle_stroke(
                            c,
                            circle_r,
                            egui::Stroke::new(3.0, Color32::from_gray(120)),
                        );
                    }
                }
            }
        }

        //
    } else if num_units == 3 || num_units == 4 {

        //
    } else {
        debug!("ams.units.len() = {:#?}", ams.units.len());
    }

    //
}

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
