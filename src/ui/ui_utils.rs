use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Rect, Response, Vec2};

pub fn put_ui(
    ui: &mut egui::Ui,
    // ui: &mut egui::Ui,
    size: Vec2,
    add_contents: impl FnOnce(&mut egui::Ui) -> Response,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());

    let layout = egui::Layout::left_to_right(egui::Align::Center);

    let mut ui = ui.child_ui(rect, layout);

    ui.set_max_size(size);

    // ui.visuals_mut().widgets.active.bg_fill = Color32::RED;
    // ui.visuals_mut().widgets.inactive.bg_fill = Color32::GREEN;
    egui::Frame::none()
        // .fill(Color32::BLUE)
        .inner_margin(0.)
        .outer_margin(0.)
        .show(&mut ui, |ui| {
            ui.set_max_size(size);
            let resp = add_contents(ui);
            ui.allocate_space(ui.available_size());
            resp
        })
        .response

    // response
    // ui.allocate_ui_at_rect(rect, |ui| {
    //     //
    // })
    // .response
}
