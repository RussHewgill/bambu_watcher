use std::sync::Arc;

use egui::{Color32, Sense, Vec2};

use crate::{
    config::PrinterConfig,
    status::{AmsUnit, PrinterState, PrinterType},
};

pub fn icon_resume() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!("../../assets/icons8-play-96.png"))
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn icon_pause() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!(
        "../../assets/icons8-pause-squared-100.png"
    ))
    .fit_to_exact_size(Vec2::new(size, size))
    .max_width(size)
    .max_height(size)
}

pub fn icon_stop() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!(
        "../../assets/icons8-red-square-96.png"
    ))
    .fit_to_exact_size(Vec2::new(size, size))
    .max_width(size)
    .max_height(size)
}

pub fn icon_controls() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!("../../assets/icons/sliders_poly.svg"))
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

// pub fn thumbnail_print<'a>(
//     printer: Arc<PrinterConfig>,
//     printer_type: &PrinterType,
//     ctx: &'a egui::Context,
// ) -> egui::Image<'a> {
//     let src = std::env::var("TEST_IMG").unwrap();
//     egui::Image::new(src)
// }

pub fn thumbnail_printer(
    printer: Arc<PrinterConfig>,
    printer_type: &PrinterType,
    size: f32,
    ctx: &egui::Context,
) -> egui::Image<'static> {
    let src = if ctx.style().visuals.dark_mode {
        // egui::include_image!("../../assets/printer_thumbnail_x1.svg")
        match printer_type {
            PrinterType::X1 => egui::include_image!("../../assets/printer_thumbnail_x1.svg"),
            PrinterType::P1P => egui::include_image!("../../assets/printer_thumbnail_p1p.svg"),
            PrinterType::P1S => egui::include_image!("../../assets/printer_thumbnail_p1s.svg"),
            PrinterType::A1 => egui::include_image!("../../assets/printer_thumbnail_n2s.svg"),
            PrinterType::A1m => egui::include_image!("../../assets/printer_thumbnail_n1.svg"),
            PrinterType::Unknown => egui::include_image!("../../assets/printer_thumbnail_x1.svg"),
        }
    } else {
        // egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg")
        match printer_type {
            PrinterType::X1 => egui::include_image!("../../assets/printer_thumbnail_x1_dark.svg"),
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
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_chamber() -> egui::Image<'static> {
    let size = 20.;
    let src = egui::include_image!("../../assets/param_chamber_temp.svg");
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_nozzle(active: bool) -> egui::Image<'static> {
    let size = 20.;
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
    let size = 20.;
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

pub fn paint_icon(ui: &mut egui::Ui, size: f32, state: &PrinterState) {
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
    let thumbnail = egui::Image::new(src).max_width(size).max_height(size);
    ui.add(thumbnail);
}

pub fn ams_icons_single(ui: &mut egui::Ui, size: f32, wide: bool, ams: &AmsUnit) {
    let n = if wide { 4 } else { 8 };
    let size = if wide { size } else { size / 2. };
    ui.columns(n, |uis| {
        for i in 0..4 {
            paint_ams_icons(&mut uis[i], i, size, ams);
        }
    });
}

pub fn ams_icons_double(ui: &mut egui::Ui, size: f32, ams0: &AmsUnit, ams1: &AmsUnit) {
    ui.columns(8, |uis| {
        for i in 0..4 {
            paint_ams_icons(&mut uis[i], i, size / 2., ams0);
        }
        for i in 4..8 {
            paint_ams_icons(&mut uis[i], i - 4, size / 2., ams1);
        }
    });
}

fn paint_ams_icons(ui: &mut egui::Ui, i: usize, size: f32, ams: &AmsUnit) {
    let size = Vec2::splat(size);
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;
    let c = rect.center();
    // let r = rect.width() / 2.0 - 1.0;
    let r = size.x / 2.0 - 1.0;

    if let Some(slot) = ams.slots[i].as_ref() {
        painter.circle_filled(c, r, slot.color);
    } else {
        painter.circle_stroke(c, r, egui::Stroke::new(1.0, Color32::from_gray(120)));
    }
}
