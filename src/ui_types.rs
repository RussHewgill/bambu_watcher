use std::{cell::RefCell, rc::Rc, sync::Arc};

use dashmap::DashMap;

use crate::{
    client::PrinterId,
    status::{PrinterStatus, PrinterStatusReport},
};

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Main,
    Options,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Main
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    // pub(super) current_tab: Tab,
    // pub(super) input_files_splitting: Vec<PathBuf>,
    // pub(super) input_files_conversion: Vec<PathBuf>,
    // pub(super) input_files_instancing: Vec<PathBuf>,
    // pub(super) output_folder: Option<PathBuf>,
    // #[serde(skip)]
    pub current_tab: Tab,
    // #[serde(skip)]
    // pub(super) processing_rx: Option<crossbeam_channel::Receiver<crate::ProcessingEvent>>,
    // #[serde(skip)]
    // pub(super) messages: Vec<String>,
    // #[serde(skip)]
    // pub(super) start_time: Option<Instant>,
    // #[serde(skip)]
    // pub(super) loaded_instance_file: Option<LoadedInstanceFile>,
    #[serde(skip)]
    pub config: crate::config::Config,

    #[serde(skip)]
    pub printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,

    #[serde(skip)]
    pub tray: Rc<RefCell<Option<tray_icon::TrayIcon>>>,
}
