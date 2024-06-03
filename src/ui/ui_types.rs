use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{
    cloud::{
        errors::ErrorMap,
        streaming::{StreamCmd, WebcamTexture},
    },
    config::{ConfigArc, PrinterConfig},
    conn_manager::{PrinterConnCmd, PrinterConnMsg, PrinterId},
    status::bambu::PrinterStatus,
};

pub use self::projects_list::ProjectsList;

use super::plotting::Graphs;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct App {
    pub current_tab: Tab,

    #[serde(skip)]
    pub config: ConfigArc,

    #[serde(skip)]
    pub cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>>,

    #[serde(skip)]
    pub stream_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<StreamCmd>>,

    #[serde(skip)]
    pub msg_rx: Option<tokio::sync::mpsc::UnboundedReceiver<PrinterConnMsg>>,

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

    #[serde(skip)]
    pub selected_stream: Option<PrinterId>,

    #[serde(skip)]
    pub printer_config_page: PrinterConfigPage,

    pub options: AppOptions,

    #[serde(skip)]
    pub login_window: Option<AppLogin>,

    /// selected printer, show right panel when Some
    pub selected_printer_controls: Option<PrinterId>,
    // #[serde(skip)]
    // pub printer_skip: Option<PrinterSkipping>,
    #[serde(skip)]
    pub printer_textures: Arc<DashMap<PrinterId, WebcamTexture>>,
    // #[serde(skip)]
    // pub printer_texture_rxs: HashMap<PrinterId, tokio::sync::watch::Receiver<Vec<u8>>>,

    // #[serde(skip)]
    pub projects: ProjectsList,

    #[serde(skip)]
    pub graphs: Option<Graphs>,
}

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Dashboard,
    Graphs,
    Printers,
    Projects,
    Options,
    // Debugging,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Dashboard
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

// #[derive(Default)]
pub struct AppLogin {
    pub username: String,
    pub password: String,
    pub sent: bool,
}

impl Default for AppLogin {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            Self {
                username: std::env::var("CLOUD_USERNAME").unwrap(),
                password: std::env::var("CLOUD_PASSWORD").unwrap(),
                sent: false,
            }
        } else {
            Self {
                username: String::new(),
                password: String::new(),
                sent: false,
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AppOptions {
    // pub dark_mode: bool,
    pub dashboard_size: (usize, usize),
    pub selected_printer: Option<PrinterId>,
    pub selected_printer_cfg: Option<NewPrinterEntry>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            // dark_mode: false,
            dashboard_size: (4, 2),
            selected_printer: None,
            selected_printer_cfg: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct NewPrinterEntry {
    pub name: String,
    pub host: String,
    pub access_code: String,
    pub serial: String,
}

impl NewPrinterEntry {
    pub fn from_cfg(cfg: &PrinterConfig) -> Self {
        Self {
            name: cfg.name.clone(),
            host: cfg.host.clone(),
            access_code: cfg.access_code.clone(),
            serial: (*cfg.serial).clone(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct PrinterConfigPage {
    pub new_printer: NewPrinterEntry,
    /// Some(false) -> in progress
    /// Some(true) -> done
    pub syncing_printers: Option<bool>,
}

pub mod projects_list {
    use egui_data_table::DataTable;
    use serde::{Deserialize, Serialize};

    use crate::cloud::projects::TaskData;

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct ProjectsList {
        pub filter: Option<String>,
        sort: Option<(SortType, SortDir)>,
        // projects: Vec<ProjectData>,
        projects: Vec<TaskData>,
        index_map: Vec<usize>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
    pub enum SortType {
        Name,
        Status,
        PrintTime,
        Material,
        PrintDate,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
    pub enum SortDir {
        Asc,
        Desc,
    }

    impl ProjectsList {
        pub fn new(projects: Vec<TaskData>) -> Self {
            Self {
                filter: None,
                sort: None,
                index_map: (0..projects.len()).collect(),
                projects,
            }
        }

        pub fn len(&self) -> usize {
            self.projects.len()
        }

        pub fn get_project(&self, index: usize) -> Option<&TaskData> {
            self.projects.get(self.index_map[index])
        }

        pub fn sorted(&self) -> Option<(SortType, SortDir)> {
            self.sort
        }

        /// Unsorted -> Asc -> Desc
        pub fn sort_by(&mut self, sort: SortType) {
            match self.sort {
                Some((cur_sort, rev)) if cur_sort == sort => match rev {
                    SortDir::Asc => {
                        self.sort = Some((sort, SortDir::Desc));
                        self._sort_by(sort, SortDir::Desc, Self::_cmp(sort));
                    }
                    SortDir::Desc => {
                        self.unsort();
                    }
                },
                Some((cur_sort, rev)) => {
                    self.sort = Some((sort, SortDir::Asc));
                    self._sort_by(sort, SortDir::Asc, Self::_cmp(sort));
                }
                /// currently unsorted
                None => {
                    self.sort = Some((sort, SortDir::Asc));
                    self._sort_by(sort, SortDir::Asc, Self::_cmp(sort));
                }
            }
        }

        fn _cmp(sort: SortType) -> impl Fn(&TaskData, &TaskData) -> std::cmp::Ordering {
            match sort {
                SortType::Name => |a: &TaskData, b: &TaskData| a.title.cmp(&b.title),
                SortType::Status => |a: &TaskData, b: &TaskData| a.status.cmp(&b.status),
                SortType::PrintTime => |a: &TaskData, b: &TaskData| a.cost_time.cmp(&b.cost_time),
                SortType::Material => {
                    |a: &TaskData, b: &TaskData| a.weight.partial_cmp(&b.weight).unwrap()
                }
                SortType::PrintDate => |a: &TaskData, b: &TaskData| a.start_time.cmp(&b.start_time), // _ => unimplemented!(),
            }
        }

        fn unsort(&mut self) {
            self.sort = None;
            self.index_map = (0..self.projects.len()).collect();
        }

        fn _sort_by(
            &mut self,
            sort: SortType,
            reversed: SortDir,
            cmp: impl Fn(&TaskData, &TaskData) -> std::cmp::Ordering,
        ) {
            self.sort = Some((sort, reversed));

            if reversed == SortDir::Desc {
                // self.projects.sort_by(|a, b| cmp(a, b).reverse());
                self.index_map
                    .sort_by(|a, b| cmp(&self.projects[*a], &self.projects[*b]).reverse());
            } else {
                self.index_map
                    .sort_by(|a, b| cmp(&self.projects[*a], &self.projects[*b]));
            }
        }

        #[cfg(feature = "nope")]
        pub fn sort_date(&mut self) {
            match self.sort {
                Some((SortType::Date, true)) => {
                    self.projects
                        .sort_by(|a, b| b.create_time.cmp(&a.create_time));
                    self.sort = Some((SortType::Date, false));
                }
                Some((SortType::Date, false)) => {
                    self.projects
                        .sort_by(|a, b| a.create_time.cmp(&b.create_time));
                    self.sort = Some((SortType::Date, true));
                }
                _ => {
                    self.projects
                        .sort_by(|a, b| a.create_time.cmp(&b.create_time));
                    self.sort = Some((SortType::Date, true));
                }
            }
        }

        #[cfg(feature = "nope")]
        pub fn sort_name(&mut self) {
            // match self.sort {
            //     Some((0, true)) => todo!(),
            //     Some((0, false)) => todo!(),
            //     _ => todo!(),
            // }

            // self.projects.sort_by(|a, b| {
            //     if reverse {
            //         b.name.cmp(&a.name)
            //     } else {
            //         a.name.cmp(&b.name)
            //     }
            // });
            // self.sort = Some((0, reverse));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum FilamentSwapStep {
    Idling,
    HeatNozzle,
    CutFilament,
    PullBackCurrentFilament,
    PushNewFilament,
    PurgeOldFilament,
    FeedFilament,
    ConfirmExtruded,
    CheckFilamentPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum AmsState {
    /// 0
    Idle,
    /// 1
    FilamentChange(FilamentSwapStep),
    /// 2
    RfidIdentifying,
    /// 3
    Assist,
    /// 4
    Calibration,
    /// 0x10
    SelfCheck,
    /// 0x20
    Debug,
    /// 0xFF
    Unknown,
}

#[cfg(feature = "nope")]
pub mod projects_list {
    use egui_data_table::DataTable;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct ProjectsList {
        pub filter: Option<String>,
        sort: Option<(SortType, bool)>,
        #[serde(
            serialize_with = "serialize_datatable",
            deserialize_with = "deserialize_datatable"
        )]
        // pub projects: Vec<crate::cloud::projects::ProjectData>,
        pub projects: DataTable<crate::cloud::projects::ProjectData>,
    }

    #[derive(Debug, Clone, Copy, Deserialize, Serialize)]
    enum SortType {
        Date,
        Name,
        PrintTime,
        Material,
    }

    fn serialize_datatable<S>(
        data: &egui_data_table::DataTable<crate::cloud::projects::ProjectData>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // let mut data = data.clone();
        let vec: Vec<crate::cloud::projects::ProjectData> = data.iter().cloned().collect();
        vec.serialize(serializer)
    }

    fn deserialize_datatable<'de, D>(
        deserializer: D,
    ) -> Result<egui_data_table::DataTable<crate::cloud::projects::ProjectData>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec: Vec<crate::cloud::projects::ProjectData> = Vec::deserialize(deserializer)?;
        Ok(egui_data_table::DataTable::new())
    }

    impl ProjectsList {
        pub fn new(projects: Vec<crate::cloud::projects::ProjectData>) -> Self {
            let mut projects_data = egui_data_table::DataTable::new();
            projects_data.replace(projects);
            Self {
                filter: None,
                sort: None,
                projects: projects_data,
            }
        }
    }

    #[cfg(feature = "nope")]
    impl ProjectsList {
        pub fn new(projects: Vec<crate::cloud::projects::ProjectData>) -> Self {
            let mut projects = egui_data_table::DataTable::new();
            Self {
                filter: None,
                sort: None,
                projects,
            }
        }

        pub fn sorted(&self) -> Option<(SortType, bool)> {
            self.sort
        }

        pub fn sort_date(&mut self) {
            match self.sort {
                Some((SortType::Date, true)) => {
                    self.projects
                        .sort_by(|a, b| b.create_time.cmp(&a.create_time));
                    self.sort = Some((SortType::Date, false));
                }
                Some((SortType::Date, false)) => {
                    self.projects
                        .sort_by(|a, b| a.create_time.cmp(&b.create_time));
                    self.sort = Some((SortType::Date, true));
                }
                _ => {
                    self.projects
                        .sort_by(|a, b| a.create_time.cmp(&b.create_time));
                    self.sort = Some((SortType::Date, true));
                }
            }
        }

        pub fn sort_name(&mut self) {
            // match self.sort {
            //     Some((0, true)) => todo!(),
            //     Some((0, false)) => todo!(),
            //     _ => todo!(),
            // }

            // self.projects.sort_by(|a, b| {
            //     if reverse {
            //         b.name.cmp(&a.name)
            //     } else {
            //         a.name.cmp(&b.name)
            //     }
            // });
            // self.sort = Some((0, reverse));
        }
    }
}
