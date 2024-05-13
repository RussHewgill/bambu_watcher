use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use dashmap::DashMap;

use crate::{client::PrinterId, status::PrinterStatus};

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Main,
    Graphs,
    Printers,
    Options,
    Debugging,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Main
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    pub current_tab: Tab,

    #[serde(skip)]
    pub config: crate::config::Config,

    #[serde(skip)]
    pub printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,

    #[serde(skip)]
    pub tray: Rc<RefCell<Option<tray_icon::TrayIcon>>>,

    pub debug_host: String,
    pub debug_serial: String,
    pub debug_code: String,

    pub printer_order: HashMap<(usize, usize), PrinterId>,
    #[serde(skip)]
    pub unplaced_printers: Vec<PrinterId>,

    pub selected_ams: HashMap<PrinterId, usize>,
}
