use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::collections::HashMap;

use bambulab::Message;
use image::Rgba;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::config::Configs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusIcon {
    Idle,
    PrintingNormally,
    PrintingError,
    Disconnected,
}

pub struct State {
    tray_icon: Option<TrayIcon>,
    icons: HashMap<StatusIcon, tray_icon::Icon>,
    rx: tokio::sync::broadcast::Receiver<Message>,

    menu_ids: HashMap<MenuId, AppCommand>,

    config: Configs,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    Reload,
    Quit,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    // Quit,
    TrayEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
}

impl State {
    pub fn new(rx: tokio::sync::broadcast::Receiver<Message>) -> Self {
        let mut icons = HashMap::new();

        let (base, width, height) = _load_icon(std::path::Path::new("icon.png"));

        let idle = tray_icon::Icon::from_rgba(base.clone().into_raw(), width, height).unwrap();
        icons.insert(StatusIcon::Idle, idle);

        let printing = imageproc::drawing::draw_filled_circle(&base.clone(), (8, 8), 6, Rgba([0, 255, 0, 255]));
        let printing = tray_icon::Icon::from_rgba(printing.into_raw(), width, height).unwrap();
        icons.insert(StatusIcon::PrintingNormally, printing);

        let error = imageproc::drawing::draw_filled_circle(&base.clone(), (8, 8), 6, Rgba([255, 0, 0, 255]));
        let error = tray_icon::Icon::from_rgba(error.into_raw(), width, height).unwrap();
        icons.insert(StatusIcon::PrintingError, error);

        let config = serde_yaml::from_reader(std::fs::File::open("config.yaml").unwrap()).unwrap();

        Self {
            tray_icon: None,
            icons,
            rx,

            config,
            menu_ids: HashMap::new(),
        }
    }

    pub fn set_icon(&self, state: StatusIcon) {
        let icon = self.icons.get(&state).unwrap();
        self.tray_icon
            .as_ref()
            .unwrap()
            .set_icon(Some(icon.clone()))
            .unwrap();
    }
}

impl ApplicationHandler<AppEvent> for State {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        //
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvent) {
        // debug!("got event: {:?}", event);
        match event {
            AppEvent::TrayEvent(ev) => {
                debug!("tray event: {:?}", ev);
            }
            AppEvent::MenuEvent(ev) => match self.menu_ids.get(ev.id()) {
                Some(AppCommand::Reload) => {
                    debug!("reload");
                }
                Some(AppCommand::Quit) => {
                    debug!("quit");
                    event_loop.exit();
                }
                None => {
                    debug!("unknown menu event: {:?}", ev);
                }
            },
        }
        //
    }

    fn new_events(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, cause: winit::event::StartCause) {
        if matches!(cause, winit::event::StartCause::Init) {
            debug!("init");
            /// Icon by https://www.flaticon.com/authors/freepik
            let icon = self
                .icons
                .get(&StatusIcon::PrintingNormally)
                .unwrap()
                .clone();

            let mut menu = Menu::new();
            let reload = MenuItem::new("Reload config", true, None);
            self.menu_ids
                .insert(reload.id().clone(), AppCommand::Reload);
            menu.append(&reload).unwrap();
            menu.append(&PredefinedMenuItem::separator()).unwrap();

            for printer in &self.config.printers {
                let item = MenuItem::new(&printer.name, true, None);
                menu.append(&item).unwrap();
            }

            menu.append(&PredefinedMenuItem::separator()).unwrap();
            let quit = MenuItem::new("Quit", true, None);
            self.menu_ids.insert(quit.id().clone(), AppCommand::Quit);
            menu.append(&quit).unwrap();

            // We create the icon once the event loop is actually running
            // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
            self.tray_icon = Some(
                TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip("winit - awesome windowing lib")
                    .with_icon(icon)
                    .with_title("x")
                    .build()
                    .unwrap(),
            );

            //
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
