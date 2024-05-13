use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
