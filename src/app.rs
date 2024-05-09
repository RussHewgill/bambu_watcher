use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{Receiver, Sender};

use bambulab::Message;
use image::Rgba;
use tray_icon::{
    menu::{AboutMetadata, IconMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, Position},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

use crate::{
    app_types::{AppCommand, AppEvent, PrinterMenu, State, StatusIcon},
    client::{PrinterConnCmd, PrinterConnMsg},
    config::{Configs, PrinterConfig},
    status::PrinterStatus,
};

/// init
impl State {
    pub fn new(config: &Configs, cmd_tx: Sender<PrinterConnCmd>) -> Self {
        let icons = Self::make_icons();

        let mut printer_status = HashMap::new();
        for printer in &config.printers {
            printer_status.insert(printer.serial.clone(), PrinterStatus::Disconnected);
        }

        Self {
            // context: egui::Context::default(),
            window: None,

            tray_icon: None,
            icons,
            // msg_rx: rx,
            config: config.clone(),

            // printers: vec![],
            printer_status,

            menu_ids: HashMap::new(),
        }
    }

    fn make_icons() -> HashMap<StatusIcon, (tray_icon::Icon, tray_icon::menu::Icon)> {
        let mut icons = HashMap::new();

        let (base, width, height) = _load_icon(std::path::Path::new("icon.png"));

        let center = (8, 8);
        let radius = 6;

        icons.insert(StatusIcon::Idle, Self::make_icon(&base, None, center, radius).unwrap());

        icons.insert(
            StatusIcon::PrintingNormally,
            Self::make_icon(&base, Some(Rgba([0, 255, 0, 255])), center, radius).unwrap(),
        );

        icons.insert(
            StatusIcon::PrintingError,
            Self::make_icon(&base, Some(Rgba([255, 0, 0, 255])), center, radius).unwrap(),
        );

        icons.insert(
            StatusIcon::Disconnected,
            Self::make_icon(&base, Some(Rgba([255, 255, 0, 255])), center, radius).unwrap(),
        );

        icons
    }

    fn make_icon(
        base: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        color: Option<Rgba<u8>>,
        pos: (i32, i32),
        radius: i32,
    ) -> Result<(tray_icon::Icon, tray_icon::menu::Icon)> {
        let (width, height) = base.dimensions();

        let base = if let Some(color) = color {
            imageproc::drawing::draw_filled_circle(&base.clone(), pos, radius, color)
        } else {
            base.clone()
        };

        let icon1 = image::ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        let color = color.unwrap_or(Rgba([0, 0, 0, 0]));
        let icon1 = imageproc::drawing::draw_filled_circle(
            &icon1,
            (width as i32 / 2, height as i32 / 2),
            width as i32 / 2,
            color,
        );
        let icon1 = tray_icon::menu::Icon::from_rgba(icon1.into_raw(), width, height)?;

        // let rgba = base.clone().into_raw();
        let icon0 = tray_icon::Icon::from_rgba(base.clone().into_raw(), width, height)?;
        // let icon1 = tray_icon::menu::Icon::from_rgba(base.clone().into_raw(), width, height)?;
        Ok((icon0, icon1))
    }
}

impl State {
    // pub fn spawn_window(&mut self, event_loop: &ActiveEventLoop) {
    //     let attributes = Window::default_attributes()
    //         .with_title("parent window")
    //         .with_position(Position::Logical(LogicalPosition::new(0.0, 0.0)))
    //         .with_inner_size(LogicalSize::new(640.0f32, 480.0f32));
    //     let window = event_loop.create_window(attributes).unwrap();
    //     self.window = Some(window);
    //     // let id = window.id();
    // }

    #[must_use]
    pub fn rebuild_menu(&mut self) -> Menu {
        let mut menu = Menu::new();
        let reload = MenuItem::new("Reload config", true, None);
        self.menu_ids
            .insert(reload.id().clone(), AppCommand::Reload);
        menu.append(&reload).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();

        for printer in &self.config.printers {
            let printer_menu = PrinterMenu::new(printer);

            // let icon = self.icons.get(&StatusIcon::Disconnected).unwrap().clone();
            let status = self
                .printer_status
                .get(&printer.serial)
                .unwrap_or(&PrinterStatus::Disconnected);
            let icon = self.icons.get(&status.to_icon()).unwrap().clone();
            let item = IconMenuItem::with_id(&printer_menu.id, &printer.name, true, Some(icon.1), None);

            let item_time_left = match status {
                PrinterStatus::Printing(done) => {
                    debug!("setting time left for printer: {:?}", printer.serial);
                    let dt = done.checked_duration_since(Instant::now()).unwrap();

                    let time_left = format!("{}:{}", dt.as_secs() / 3600, dt.as_secs() % 3600 / 60);
                    MenuItem::with_id(&printer_menu.id_time_left, time_left, false, None)
                }
                _ => {
                    debug!("no time left for printer: {:?}", printer.serial);
                    MenuItem::with_id(&printer_menu.id_time_left, "--:--:--", false, None)
                }
            };
            let item_eta = MenuItem::with_id(&printer_menu.id_time_left, "--:--:--", false, None);

            menu.append(&item).unwrap();
            menu.append(&item_time_left).unwrap();
            menu.append(&item_eta).unwrap();

            menu.append(&PredefinedMenuItem::separator()).unwrap();
        }

        let quit = MenuItem::new("Quit", true, None);
        self.menu_ids.insert(quit.id().clone(), AppCommand::Quit);
        menu.append(&quit).unwrap();

        menu
    }

    pub fn set_icon(&self, state: StatusIcon) {
        let icon = self.icons.get(&state).unwrap();
        self.tray_icon
            .as_ref()
            .unwrap()
            .set_icon(Some(icon.0.clone()))
            .unwrap();
    }

    // pub fn set_tooltip(&mut self) {
    //     if let Some(tray) = self.tray_icon.as_mut() {
    //         // tray.set_tooltip(Some(tooltip)).unwrap();
    //     }
    // }

    #[cfg(feature = "nope")]
    pub fn set_printer_icon(&self, id: &str, state: StatusIcon) {
        // let icon = self.icons.get(&state).unwrap();
        // self.tray_icon
        //     .as_ref()
        //     .unwrap()
        //     .set_menu_icon(&id, Some(icon.1.clone()))
        //     .unwrap();
    }
}

impl ApplicationHandler<AppEvent> for State {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        debug!("resumed");
        // // We add delay of 16 ms (60fps) to event_loop to reduce cpu load.
        // // This can be removed to allow ControlFlow::Poll to poll on each cpu cycle
        // // Alternatively, you can set ControlFlow::Wait or use TrayIconEvent::set_event_handler,
        // // see https://github.com/tauri-apps/tray-icon/issues/83#issuecomment-1697773065
        // event_loop.set_control_flow(ControlFlow::WaitUntil(
        //     std::time::Instant::now() + std::time::Duration::from_millis(16),
        // ));

        //
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        //
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        // debug!("got event: {:?}", event);
        match event {
            AppEvent::TrayEvent(ev) => {
                debug!("tray event: {:?}", ev);
                for (id, status) in self.printer_status.iter_mut() {
                    let done = Instant::now() + Duration::from_secs((60 + 23) * 60);
                    debug!("setting printer status: {:?} = {:?}", id, done);
                    *status = PrinterStatus::Printing(done);
                }
                let menu = self.rebuild_menu();
                if let Some(tray) = self.tray_icon.as_mut() {
                    tray.set_menu(Some(Box::new(menu)));
                }
            }
            AppEvent::MenuEvent(ev) => match self.menu_ids.get(ev.id()) {
                Some(AppCommand::Reload) => {
                    debug!("reload");
                    // self.spawn_window(event_loop);
                }
                Some(AppCommand::Quit) => {
                    debug!("quit");
                    event_loop.exit();
                }
                None => {
                    debug!("unknown menu event: {:?}", ev);
                }
            },
            AppEvent::ConnMsg(msg) => {
                debug!("conn msg: {:?}", msg);
                match msg {
                    PrinterConnMsg::StatusReport(id, status) => {
                        debug!("got status report from printer: {:?}", id);
                        match status.status {
                            PrinterStatus::Idle => {
                                // self.set_printer_icon(&id, StatusIcon::Idle);
                            }
                            PrinterStatus::Printing(_) => {
                                // self.set_printer_icon(&id, StatusIcon::PrintingNormally);
                            }
                            PrinterStatus::Error(_) => {
                                // self.set_printer_icon(&id, StatusIcon::PrintingError);
                            }
                            PrinterStatus::Paused => todo!(),
                            PrinterStatus::Disconnected => todo!(),
                        }
                    }
                }
            }
        }
        //
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        // #[cfg(feature = "nope")]
        if matches!(cause, winit::event::StartCause::Init) {
            debug!("init");
            /// Icon by https://www.flaticon.com/authors/freepik
            let icon = self
                .icons
                .get(&StatusIcon::PrintingNormally)
                .unwrap()
                .clone();

            let menu = self.rebuild_menu();

            // We create the icon once the event loop is actually running
            // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
            self.tray_icon = Some(
                TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip("winit - awesome windowing lib")
                    .with_icon(icon.0)
                    .with_title("x")
                    .build()
                    .unwrap(),
            );

            //
        }
    }
}

impl PrinterMenu {
    pub fn new(cfg: &PrinterConfig) -> Self {
        Self {
            id: cfg.serial.clone(),
            id_time_left: format!("{}_time_left", cfg.serial),
            id_eta: format!("{}_eta", cfg.serial),
        }
    }
}

fn _load_icon(path: &std::path::Path) -> (image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, u32, u32) {
    let image = image::open(path)
        .expect("Failed to open icon path")
        .into_rgba8();
    let (width, height) = image.dimensions();

    (image, width, height)
}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
