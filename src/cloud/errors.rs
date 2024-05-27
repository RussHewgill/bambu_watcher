use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write};

const ERRORS_URL: &'static str = "https://e.bambulab.com/query.php?lang=en";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorMap {
    device: std::collections::HashMap<String, String>,
    hms: std::collections::HashMap<String, String>,
}

impl ErrorMap {
    fn from_errors(errors: ErrorCodes) -> Self {
        let mut device = std::collections::HashMap::new();
        let mut hms = std::collections::HashMap::new();

        for e in errors.device_error {
            device.insert(e.ecode, e.intro);
        }

        for e in errors.device_hms {
            hms.insert(e.ecode, e.intro);
        }

        Self { device, hms }
    }
}

/// see also:
/// https://github.com/greghesp/ha-bambulab/blob/main/custom_components/bambu_lab/pybambu/const.py
pub async fn fetch_error_codes() -> Result<()> {
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let res = client.get(ERRORS_URL).send().await?;

    if !res.status().is_success() {
        debug!("res {:#?}", res);
        bail!("Failed to get response, url = {}", ERRORS_URL);
    }

    let json: serde_json::Value = res.json().await?;

    // debug!("json {:#?}", json);

    let s = serde_json::to_string_pretty(&json)?;

    /// write s to file
    let mut file = File::create("errors.json")?;
    file.write_all(s.as_bytes())?;

    Ok(())
}

pub fn read_error_codes() -> Result<ErrorCodes> {
    let file = File::open("errors.json")?;
    let reader = std::io::BufReader::new(file);
    let codes: ErrorCodes = serde_json::from_reader(reader)?;

    Ok(codes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCodes {
    pub device_error: Vec<ErrorCode>,
    pub device_hms: Vec<ErrorCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCode {
    pub ecode: String,
    pub intro: String,
}
