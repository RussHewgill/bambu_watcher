use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::conn_manager::PrinterId;

#[derive(Clone)]
pub struct ConfigArc(Arc<RwLock<Config>>);

impl Default for ConfigArc {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Config {
            printers: HashMap::new(),
        })))
    }
}

impl ConfigArc {
    pub fn new(config: Config) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    pub fn add_printer(&mut self, printer: Arc<PrinterConfig>) {
        for (id, p) in self.0.read().printers.iter() {
            if *id == printer.serial {
                error!("Duplicate printer serial");
                return;
            }
            if p.host == printer.host {
                error!("Duplicate printer host");
                return;
            }
        }
        // self.0.write().printers.push(Arc::new(printer));
        self.0
            .write()
            .printers
            // .insert(printer.serial.clone(), Arc::new(printer));
            .insert(printer.serial.clone(), printer);
    }

    // pub fn printers(&self) -> Vec<Arc<PrinterConfig>> {
    //     self.0.read().printers.clone()
    // }

    pub fn printers(&self) -> Vec<Arc<PrinterConfig>> {
        self.0.read().printers.values().cloned().collect()
    }

    // pub fn printer_ids(&self) -> impl Iterator<Item = &str> {
    //     // self.0.read().printers.iter().map(|p| p.serial.as_str())
    // }

    pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<PrinterConfig>> {
        self.0.read().printers.get(serial).cloned()
    }
}

// #[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct Config {
    // pub printers: Vec<Arc<PrinterConfig>>,
    printers: HashMap<PrinterId, Arc<PrinterConfig>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub printers: Vec<PrinterConfig>,
}

impl Config {
    pub fn read_from_file(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config: ConfigFile = serde_yaml::from_reader(reader)?;

        let mut out = Self {
            printers: HashMap::new(),
        };

        for mut printer in config.printers.into_iter() {
            out.printers
                .insert(printer.serial.clone(), Arc::new(printer));
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub name: String,
    pub host: String,
    pub access_code: String,
    // #[serde(skip)]
    pub serial: Arc<String>,
}
