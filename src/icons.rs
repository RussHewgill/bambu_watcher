use egui::Vec2;

use crate::status::PrinterState;

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

pub fn thumbnail_printer() -> egui::Image<'static> {
    let size = 80.;
    egui::Image::new(egui::include_image!("../assets/printer_thumbnail_x1.svg"))
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
