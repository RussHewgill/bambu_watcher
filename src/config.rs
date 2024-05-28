use anyhow::{anyhow, bail, ensure, Context, Result};
use dashmap::DashMap;
use tracing::{debug, error, info, trace, warn};

use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, Arc},
};

// use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{conn_manager::PrinterId, ui::ui_types::NewPrinterEntry};

#[derive(Clone)]
// pub struct ConfigArc(Arc<RwLock<Config>>);
pub struct ConfigArc {
    // pub config: Arc<RwLock<Config>>,
    config: Config,
    pub auth: Arc<RwLock<crate::auth::AuthDb>>,
    pub logged_in: Arc<AtomicBool>,
}

impl Default for ConfigArc {
    fn default() -> Self {
        warn!("config default shouldn't be used");
        // Self(Arc::new(RwLock::new(Config {
        //     logged_in: false,
        //     auth: crate::auth::AuthDb::empty(),
        //     printers: HashMap::new(),
        // })))
        Self {
            // config: Arc::new(RwLock::new(Config {
            //     logged_in: false,
            //     printers: HashMap::new(),
            // })),
            config: Config {
                ids: Arc::new(RwLock::new(HashSet::new())),
                printers: Arc::new(DashMap::new()),
            },
            auth: Arc::new(RwLock::new(crate::auth::AuthDb::empty())),
            logged_in: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// new, auth
impl ConfigArc {
    pub fn new(config: Config, mut auth: crate::auth::AuthDb) -> Self {
        let logged_in = auth.get_token().ok().flatten().is_some();
        Self {
            // config: Arc::new(RwLock::new(config)),
            config,
            auth: Arc::new(RwLock::new(auth)),
            logged_in: Arc::new(AtomicBool::new(logged_in)),
        }
    }

    pub fn logged_in(&self) -> bool {
        // self.config.blocking_read().logged_in
        // warn!("TODO: logged_in");
        // false
        self.logged_in.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_logged_in(&self, logged_in: bool) {
        self.logged_in
            .store(logged_in, std::sync::atomic::Ordering::Relaxed);
    }

    pub async fn get_token_async(&self) -> Result<Option<crate::auth::Token>> {
        {
            let token = self.auth.read().await.get_token_cached();
            if let Some(token) = token {
                return Ok(Some(token));
            }
        }

        self.auth.write().await.get_token()
    }

    pub async fn fetch_new_token(&self, username: &str, password: &str) -> Result<()> {
        self.auth
            .write()
            .await
            .login_and_get_token(username, password)
            .await?;
        Ok(())
    }
}

impl ConfigArc {
    pub async fn add_printer(&self, printer: Arc<RwLock<PrinterConfig>>) {
        let id = printer.read().await.serial.clone();
        // self.config.printers.insert(id, printer);
        self.config.ids.write().await.insert(id.clone());
        self.config.printers.insert(id, printer);
    }

    pub async fn update_printer(&self, id: &PrinterId, cfg: &NewPrinterEntry) {
        let mut config = self.config.printers.get(id).unwrap().clone();
        let mut config = config.write().await;
        config.name = cfg.name.clone();
        config.host = cfg.host.clone();
        config.access_code = cfg.access_code.clone();
        // config.color = cfg.color;
    }

    #[cfg(feature = "nope")]
    pub fn printers_ref(&self) -> Vec<&PrinterConfig> {
        self.config
            .printers
            .iter()
            .map(|p| p.value().clone())
            .collect()
    }

    pub fn printer_ids(&self) -> Vec<PrinterId> {
        // self.config.printers.keys().cloned().collect()
        self.config.ids.blocking_read().iter().cloned().collect()
    }

    pub async fn printer_ids_async(&self) -> Vec<PrinterId> {
        self.config.ids.read().await.iter().cloned().collect()
    }

    pub fn printers(&self) -> Vec<Arc<RwLock<PrinterConfig>>> {
        self.config
            .printers
            .iter()
            .map(|v| v.value().clone())
            .collect()
    }

    #[cfg(feature = "nope")]
    /// will get stale after config update, so will need to restart the printer's connection
    pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<PrinterConfig>> {
        if let Some(p) = self.config.printers.get(serial) {
            Some(p.clone())
        } else {
            None
        }
    }

    pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<RwLock<PrinterConfig>>> {
        self.config.printers.get(serial).map(|v| v.clone())
    }
}

#[derive(Clone)]
pub struct Config {
    // printers: HashMap<PrinterId, Arc<PrinterConfig>>,
    // printers: DashMap<PrinterId, PrinterConfig>,
    // printers: Arc<DashMap<PrinterId, RwLock<PrinterConfig>>>,
    // printers: Arc<DashMap<PrinterId, Arc<RwLock<PrinterConfig>>>>,
    ids: Arc<RwLock<HashSet<PrinterId>>>,
    printers: Arc<DashMap<PrinterId, Arc<RwLock<PrinterConfig>>>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    printers: Vec<PrinterConfig>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            ids: Arc::new(RwLock::new(HashSet::new())),
            printers: Arc::new(DashMap::new()),
        }
    }

    pub fn read_from_file(path: &str) -> Result<(Self, crate::auth::AuthDb)> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let config: ConfigFile = serde_yaml::from_reader(reader)?;

        let mut auth = crate::auth::AuthDb::read_or_create()?;

        let logged_in = matches!(auth.get_token(), Ok(Some(_)));

        let mut ids = HashSet::new();
        let mut printers = DashMap::new();

        for mut printer in config.printers.into_iter() {
            ids.insert(printer.serial.clone());
            printers.insert(printer.serial.clone(), Arc::new(RwLock::new(printer)));
        }

        let mut out = Self {
            ids: Arc::new(RwLock::new(ids)),
            // logged_in,
            // auth,
            printers: Arc::new(printers),
        };

        Ok((out, auth))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub name: String,
    #[serde(default)]
    // pub host: Option<String>,
    pub host: String,
    pub access_code: String,
    // #[serde(skip)]
    pub serial: Arc<String>,
    // #[serde(default)]
    // // pub cloud: bool,
    // pub cloud: std::sync::atomic::AtomicBool,
    #[serde(default)]
    pub color: [u8; 3],
}

impl PrinterConfig {
    pub fn from_device(id: PrinterId, device: &crate::cloud::cloud_types::Device) -> Self {
        Self {
            name: device.name.clone(),
            // host: device.host,
            host: "".to_string(),
            // host: None,
            access_code: device.dev_access_code.clone(),
            serial: id,
            color: [0, 0, 0],
        }
    }
}

#[cfg(feature = "nope")]
mod old {

    #[derive(Clone)]
    // pub struct ConfigArc(Arc<RwLock<Config>>);
    pub struct ConfigArc {
        pub config: Arc<RwLock<Config>>,
        pub auth: Arc<RwLock<crate::auth::AuthDb>>,
    }

    impl Default for ConfigArc {
        fn default() -> Self {
            warn!("config default shouldn't be used");
            // Self(Arc::new(RwLock::new(Config {
            //     logged_in: false,
            //     auth: crate::auth::AuthDb::empty(),
            //     printers: HashMap::new(),
            // })))
            Self {
                config: Arc::new(RwLock::new(Config {
                    logged_in: false,
                    printers: HashMap::new(),
                })),
                auth: Arc::new(RwLock::new(crate::auth::AuthDb::empty())),
            }
        }
    }

    /// new, auth
    impl ConfigArc {
        pub fn new(config: Config, auth: crate::auth::AuthDb) -> Self {
            // Self(Arc::new(RwLock::new(config)))
            Self {
                config: Arc::new(RwLock::new(config)),
                auth: Arc::new(RwLock::new(auth)),
            }
        }

        pub fn logged_in(&self) -> bool {
            self.config.blocking_read().logged_in
        }

        pub async fn logged_in_async(&self) -> bool {
            self.config.read().await.logged_in
        }

        pub async fn get_token_async(&self) -> Result<Option<crate::auth::Token>> {
            {
                let token = self.auth.read().await.get_token_cached();
                if let Some(token) = token {
                    return Ok(Some(token));
                }
            }

            self.auth.write().await.get_token()
        }

        #[cfg(feature = "nope")]
        pub fn get_token(&self) -> Result<Option<crate::auth::Token>> {
            {
                let token = self.auth.blocking_read().get_token_cached();
                if let Some(token) = token {
                    return Ok(Some(token));
                }
            }

            self.auth.blocking_write().get_token()
        }

        pub async fn fetch_new_token(&self, username: &str, password: &str) -> Result<()> {
            self.auth
                .write()
                .await
                .login_and_get_token(username, password)
                .await?;
            Ok(())
        }
    }

    /// printers
    impl ConfigArc {
        pub fn add_printer(&mut self, printer: Arc<PrinterConfig>) {
            {
                for (id, p) in self.config.blocking_read().printers.iter() {
                    if *id == printer.serial {
                        error!("Duplicate printer serial");
                        return;
                    }
                    if p.host == printer.host {
                        error!("Duplicate printer host");
                        return;
                    }
                }
            }
            // self.0.write().printers.push(Arc::new(printer));
            self.config
                .blocking_write()
                .printers
                // .insert(printer.serial.clone(), Arc::new(printer));
                .insert(printer.serial.clone(), printer);
        }

        pub async fn printers_async(&self) -> Vec<Arc<PrinterConfig>> {
            self.config
                .read()
                .await
                .printers
                .values()
                .cloned()
                .collect()
        }

        pub fn printers(&self) -> Vec<Arc<PrinterConfig>> {
            self.config
                .blocking_read()
                .printers
                .values()
                .cloned()
                .collect()
        }

        pub async fn get_printer_async(&self, serial: &PrinterId) -> Option<Arc<PrinterConfig>> {
            self.config.read().await.printers.get(serial).cloned()
        }

        pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<PrinterConfig>> {
            self.config.blocking_read().printers.get(serial).cloned()
        }

        pub fn update_printer_cfg(&self, cfg: &PrinterConfig) {
            let mut config = self.config.blocking_write();
            // config.printers.insert(cfg.serial.clone(), Arc::new(cfg.clone()));
            unimplemented!()
        }
    }
}
