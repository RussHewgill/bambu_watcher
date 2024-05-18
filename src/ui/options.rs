use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::ViewportBuilder;
use egui_phosphor::fill;
use tracing::{debug, error, info, trace, warn};

use crate::ui::ui_types::{App, AppLogin, GridLocation};

/// display
impl App {
    pub fn show_options(&mut self, ui: &mut egui::Ui) {
        // ui.label("TODO: Options");

        if self.config.logged_in() {
            if ui.button("Logout").clicked() {
                let _ = self
                    .cmd_tx
                    .as_ref()
                    .unwrap()
                    .send(crate::conn_manager::PrinterConnCmd::Logout);
            }
        } else {
            ui.label("Not logged in");
            if self.login_window.is_some() {
                self.show_login(ui);
            } else if ui.button("Login").clicked() {
                self.login_window = Some(AppLogin::default());
            }
        }

        ui.separator();

        egui::Grid::new("options_grid").show(ui, |ui| {
            egui::widgets::global_dark_light_mode_buttons(ui);
            ui.end_row();

            ui.label("Rows");

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_UP)),
                )
                .clicked()
            {
                self.change_rows(false);
            }
            ui.add_sized(
                ui.available_size(),
                egui::Label::new(&format!("{:?}", self.options.dashboard_size.1)),
            );

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_DOWN)),
                )
                .clicked()
            {
                self.change_rows(true);
            }

            ui.end_row();

            ui.label("Columns");
            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_LEFT)),
                )
                .clicked()
            {
                self.change_columns(false);
            }
            ui.add_sized(
                ui.available_size(),
                egui::Label::new(&format!("{:?}", self.options.dashboard_size.0)),
            );

            if ui
                .add_sized(
                    ui.available_size(),
                    egui::Button::new(&format!("{}", fill::ARROW_FAT_RIGHT)),
                )
                .clicked()
            {
                self.change_columns(true);
            }

            ui.end_row();
        });
    }

    fn show_login(&mut self, ui: &mut egui::Ui) {
        let Some(login_window) = self.login_window.as_mut() else {
            return;
        };

        egui::Grid::new("login_grid").show(ui, |ui| {
            ui.label("Username");
            ui.text_edit_singleline(&mut login_window.username);
            ui.end_row();

            ui.label("Password");
            ui.text_edit_singleline(&mut login_window.password);
            ui.end_row();

            ui.allocate_space(ui.available_size());
        });

        if login_window.sent {
            ui.label("Logging in...");
        } else {
            ui.horizontal(|ui| {
                let Some(login_window) = self.login_window.as_mut() else {
                    return;
                };
                if ui.button("Login").clicked() {
                    let res = self.cmd_tx.as_ref().unwrap().send(
                        crate::conn_manager::PrinterConnCmd::Login(
                            login_window.username.clone(),
                            login_window.password.clone(),
                        ),
                    );

                    if let Err(e) = res {
                        error!("Failed to send login command: {:?}", e);
                    } else {
                        login_window.sent = true;
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.login_window = None;
                    return;
                }
            });
        }

        //
    }

    #[cfg(feature = "nope")]
    fn show_login(&mut self, ui: &mut egui::Ui) {
        let builder = ViewportBuilder::default()
            .with_title("Bambu Cloud Login")
            .with_inner_size([300., 200.]);
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("login_window"),
            builder,
            |ctx, _| {
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.login_window = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    let Some(login_window) = self.login_window.as_mut() else {
                        return;
                    };
                    egui::Grid::new("login_grid").show(ui, |ui| {
                        ui.label("Username");
                        ui.text_edit_singleline(&mut login_window.username);
                        ui.end_row();

                        ui.label("Password");
                        ui.text_edit_singleline(&mut login_window.password);
                        ui.end_row();

                        ui.allocate_space(ui.available_size());
                    });

                    if ui.button("Login").clicked() {
                        //
                    }
                    if ui.button("Cancel").clicked() {
                        self.login_window = None;
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                });

                // if exit {
                //     drop(login_window);
                //     self.login_window = None;
                //     ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                //     return;
                // }
            },
        );
    }
}

/// apply options
impl App {
    pub fn move_printer(&mut self, from: &GridLocation, to: &GridLocation) {
        if from == to {
            return;
        }
        match (
            self.printer_order.remove(&from),
            self.printer_order.remove(&to),
        ) {
            (Some(id_from), Some(id_to)) => {
                // debug!("TODO: swap printers");
                self.printer_order.insert(*to, id_from);
                self.printer_order.insert(*from, id_to);
            }
            (Some(id), None) => {
                debug!("moving printer {} from {:?} to {:?}", id, from, to);
                self.printer_order.insert(*to, id);
                //
            }
            (None, _) => {
                error!("Drop: No printer at drop source");
                // self.printer_order.insert(to, from);
                // self.printer_order.remove(&from);
            }
        }
    }

    /// currently keeps printers in place even when rows/cols are hidden
    pub fn change_rows(&mut self, add: bool) {
        if add {
            self.options.dashboard_size.1 += 1;
        } else {
            if self.options.dashboard_size.1 == 1 {
                return;
            }
            // warn!("TODO: what to do if printer is in removed row?");

            // for x in 0..self.options.dashboard_size.0 {
            //     let pos = GridLocation::new(x, self.options.dashboard_size.1 - 1);
            //     if let Some(id) = self.printer_order.get(&pos) {
            //         warn!("printer {:?} at {:?}", id, pos);
            //     }
            // }

            self.options.dashboard_size.1 -= 1;
        }
    }

    pub fn change_columns(&mut self, add: bool) {
        if add {
            self.options.dashboard_size.0 += 1;
        } else {
            if self.options.dashboard_size.0 == 1 {
                return;
            }
            // warn!("TODO: what to do if printer is in removed column?");
            self.options.dashboard_size.0 -= 1;
        }
    }
}
