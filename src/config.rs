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
        warn!("config default shouldn't be used");
        Self(Arc::new(RwLock::new(Config {
            logged_in: false,
            auth: crate::auth::AuthDb::empty(),
            printers: HashMap::new(),
        })))
    }
}

impl ConfigArc {
    pub fn new(config: Config) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    pub fn logged_in(&self) -> bool {
        self.0.read().logged_in
    }

    pub fn read_auth(&self) {
        // let path = "auth.db";

        // let mut db = auth::AuthDb::read_or_create("auth.db")?;
    }

    pub fn get_token(&self) -> Result<Option<crate::auth::Token>> {
        if let Some(token) = self.0.read().auth.get_token_cached() {
            Ok(Some(token))
        } else {
            self.0.write().auth.get_token()
        }
    }

    pub async fn fetch_new_token(&self, username: &str, password: &str) -> Result<()> {
        // self.0.write().auth.login_and_get_token(username, password)
        unimplemented!()
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

    pub fn printers(&self) -> Vec<Arc<PrinterConfig>> {
        self.0.read().printers.values().cloned().collect()
    }

    pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<PrinterConfig>> {
        self.0.read().printers.get(serial).cloned()
    }
}

// #[derive(Debug, Default, Clone, Serialize, Deserialize)]
// #[derive(Clone)]
pub struct Config {
    logged_in: bool,
    // auth: Option<crate::auth::AuthDb>,
    auth: crate::auth::AuthDb,
    // pub printers: Vec<Arc<PrinterConfig>>,
    printers: HashMap<PrinterId, Arc<PrinterConfig>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    pub printers: Vec<PrinterConfig>,
}

impl Config {
    pub fn read_from_file(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config: ConfigFile = serde_yaml::from_reader(reader)?;

        let mut auth = crate::auth::AuthDb::read_or_create()?;

        let logged_in = matches!(auth.get_token(), Ok(Some(_)));

        let mut out = Self {
            logged_in,
            auth,
            printers: HashMap::new(),
        };

        for mut printer in config.printers.into_iter() {
            out.printers
                .insert(printer.serial.clone(), Arc::new(printer));
        }

        Ok(out)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub name: String,
    pub host: String,
    pub access_code: String,
    // #[serde(skip)]
    pub serial: Arc<String>,
}
