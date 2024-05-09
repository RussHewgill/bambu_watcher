use std::{collections::HashMap, time::Instant};

use bambulab::Message;
use tray_icon::{
    menu::{MenuEvent, MenuId},
    TrayIcon, TrayIconEvent,
};

use crate::{
    client::{PrinterConnMsg, PrinterId},
    config::Configs,
    status::PrinterStatus,
};

pub struct State {
    // pub(super) context: egui::Context,
    pub(super) window: Option<winit::window::Window>,

    pub(super) tray_icon: Option<TrayIcon>,
    pub(super) icons: HashMap<StatusIcon, (tray_icon::Icon, tray_icon::menu::Icon)>,

    // pub(super) msg_rx: tokio::sync::broadcast::Receiver<Message>,
    // pub(super) cmd_rx: tokio::sync::broadcast::Sender<Message>,
    pub(super) menu_ids: HashMap<MenuId, AppCommand>,

    // pub(super) printers: Vec<PrinterMenu>,
    pub(super) printer_status: HashMap<PrinterId, PrinterStatus>,

    pub(super) config: Configs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusIcon {
    Idle,
    PrintingNormally,
    PrintingError,
    Disconnected,
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
    ConnMsg(PrinterConnMsg),
}

#[derive(Debug, Clone)]
pub struct PrinterMenu {
    pub id: String,
    pub id_time_left: String,
    pub id_eta: String,
}
