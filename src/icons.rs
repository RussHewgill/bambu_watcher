use egui::Vec2;

use crate::{
    config::PrinterConfig,
    status::{PrinterState, PrinterType},
};

/// MARK: icons
pub fn icon_resume() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!("../assets/icons8-play-96.png"))
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn icon_pause() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!(
        "../assets/icons8-pause-squared-100.png"
    ))
    .fit_to_exact_size(Vec2::new(size, size))
    .max_width(size)
    .max_height(size)
}

pub fn icon_stop() -> egui::Image<'static> {
    let size = 20.;
    egui::Image::new(egui::include_image!("../assets/icons8-red-square-96.png"))
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_printer(
    printer: &PrinterConfig,
    printer_type: &PrinterType,
    ctx: &egui::Context,
) -> egui::Image<'static> {
    let size = 80.;

    let src = if ctx.style().visuals.dark_mode {
        // egui::include_image!("../assets/printer_thumbnail_x1.svg")
        match printer_type {
            PrinterType::X1 => egui::include_image!("../assets/printer_thumbnail_x1.svg"),
            PrinterType::P1P => egui::include_image!("../assets/printer_thumbnail_p1p.svg"),
            PrinterType::P1S => egui::include_image!("../assets/printer_thumbnail_p1s.svg"),
            PrinterType::A1 => egui::include_image!("../assets/printer_thumbnail_n2s.svg"),
            PrinterType::A1m => egui::include_image!("../assets/printer_thumbnail_n1.svg"),
            PrinterType::Unknown => egui::include_image!("../assets/printer_thumbnail_x1.svg"),
        }
    } else {
        // egui::include_image!("../assets/printer_thumbnail_x1_dark.svg")
        match printer_type {
            PrinterType::X1 => egui::include_image!("../assets/printer_thumbnail_x1_dark.svg"),
            PrinterType::P1P => egui::include_image!("../assets/printer_thumbnail_p1p_dark.svg"),
            PrinterType::P1S => egui::include_image!("../assets/printer_thumbnail_p1s_dark.svg"),
            PrinterType::A1 => egui::include_image!("../assets/printer_thumbnail_n2s_dark.svg"),
            PrinterType::A1m => egui::include_image!("../assets/printer_thumbnail_n1_dark.svg"),
            PrinterType::Unknown => egui::include_image!("../assets/printer_thumbnail_x1_dark.svg"),
        }
    };

    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_chamber() -> egui::Image<'static> {
    let size = 20.;
    let src = egui::include_image!("../assets/param_chamber_temp.svg");
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn thumbnail_nozzle(active: bool) -> egui::Image<'static> {
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

pub fn thumbnail_bed(active: bool) -> egui::Image<'static> {
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

pub fn thumbnail_fan(on: bool) -> egui::Image<'static> {
    let size = 20.;
    let src = if on {
        egui::include_image!("../assets/monitor_fan_on.svg")
    } else {
        egui::include_image!("../assets/monitor_fan_off.svg")
    };
    egui::Image::new(src)
        .fit_to_exact_size(Vec2::new(size, size))
        .max_width(size)
        .max_height(size)
}

pub fn paint_icon(ui: &mut egui::Ui, size: f32, state: &PrinterState) {
    let src = match state {
        PrinterState::Idle => egui::include_image!("../assets/icons/check-circle_poly.svg"),
        PrinterState::Paused => egui::include_image!("../assets/icons/pause-circle_poly.svg"),
        PrinterState::Printing => egui::include_image!("../assets/icons/play-circle_poly.svg"),
        PrinterState::Error(_) => {
            egui::include_image!("../assets/icons/exclamation-triangle_poly.svg")
        }
        PrinterState::Disconnected => egui::include_image!("../assets/icons/disconnected_poly.svg"),
        PrinterState::Unknown(_) => {
            egui::include_image!("../assets/icons/question-circle_poly.svg")
        }
    };

    #[cfg(feature = "nope")]
    let src = match state {
        PrinterState::Idle => {
            egui::include_image!("../assets/icons8-hourglass-100.png")
        }
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
        PrinterState::Unknown(s) => {
            ui.label(format!("Unknown: {}", &s));
            return;
        }
    };
    let thumbnail = egui::Image::new(src).max_width(size).max_height(size);
    ui.add(thumbnail);
}
