use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct ConfigArc(Arc<RwLock<Config>>);

impl ConfigArc {
    pub fn new(config: Config) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    pub fn add_printer(&mut self, printer: PrinterConfig) {
        for p in self.0.read().printers.iter() {
            if p.serial == printer.serial {
                error!("Duplicate printer serial");
                return;
            }
            if p.host == printer.host {
                error!("Duplicate printer host");
                return;
            }
        }
        self.0.write().printers.push(printer);
    }

    pub fn printers(&self) -> Vec<PrinterConfig> {
        self.0.read().printers.clone()
    }
    pub fn get_printer(&self, serial: &str) -> Option<PrinterConfig> {
        self.0
            .read()
            .printers
            .iter()
            .find(|p| p.serial == serial)
            .cloned()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub printers: Vec<PrinterConfig>,
    // pub printers: HashMap<PrinterId, PrinterConfig>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub name: String,
    pub host: String,
    pub access_code: String,
    pub serial: String,
}
