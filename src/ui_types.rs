use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    config::ConfigArc,
    conn_manager::{PrinterConnCmd, PrinterId},
    status::PrinterStatus,
};

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Main,
    Graphs,
    Printers,
    Options,
    // Debugging,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Main
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GridLocation {
    pub col: usize,
    pub row: usize,
}

impl GridLocation {
    pub fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    pub current_tab: Tab,

    #[serde(skip)]
    pub config: ConfigArc,

    #[serde(skip)]
    pub cmd_tx: Option<tokio::sync::mpsc::Sender<PrinterConnCmd>>,

    #[serde(skip)]
    pub printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,

    // #[serde(skip)]
    // pub tray: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
    pub debug_host: String,
    pub debug_serial: String,
    pub debug_code: String,

    pub printer_order: HashMap<GridLocation, PrinterId>,
    #[serde(skip)]
    pub unplaced_printers: Vec<PrinterId>,

    pub selected_ams: HashMap<PrinterId, usize>,

    // #[serde(skip)]
    pub new_printer: NewPrinterEntry,

    pub options: AppOptions,

    #[serde(skip)]
    pub login_window: Option<AppLogin>,

    #[serde(skip)]
    pub auth: Option<crate::auth::AuthDb>,
}

#[derive(Default)]
pub struct AppLogin {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct AppOptions {
    // pub dark_mode: bool,
    pub dashboard_size: (usize, usize),
    pub selected_printer: Option<PrinterId>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            // dark_mode: false,
            dashboard_size: (4, 2),
            selected_printer: None,
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct NewPrinterEntry {
    pub name: String,
    pub host: String,
    pub access_code: String,
    pub serial: String,
}
