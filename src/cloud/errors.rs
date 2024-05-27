use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Write};

const ERRORS_URL: &'static str = "https://e.bambulab.com/query.php?lang=en";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorMap {
    device: std::collections::HashMap<u64, String>,
    hms: std::collections::HashMap<u64, String>,
}

impl ErrorMap {
    pub fn get_error(&self, ecode: u64) -> Option<&str> {
        if let Some(e) = self.device.get(&ecode) {
            return Some(e);
        }

        if let Some(e) = self.hms.get(&ecode) {
            return Some(e);
        }

        None
    }

    pub async fn read_or_fetch() -> Result<Self> {
        if let Ok(errors) = Self::read_error_codes() {
            Ok(errors)
        } else {
            let out = Self::fetch().await?;
            out.save_error_codes()?;
            Ok(out)
        }
    }

    fn read_error_codes() -> Result<Self> {
        let file = File::open("errors.json")?;
        let reader = std::io::BufReader::new(file);
        let codes: Self = serde_json::from_reader(reader)?;

        Ok(codes)
    }

    fn save_error_codes(&self) -> Result<()> {
        let s = serde_json::to_string_pretty(self)?;
        let mut file = File::create("errors.json")?;
        file.write_all(s.as_bytes())?;
        Ok(())
    }

    async fn fetch() -> Result<Self> {
        let errors = fetch_error_codes().await?;
        let out = Self::from_errors(errors);
        out.save_error_codes()?;
        Ok(out)
    }

    fn from_errors(errors: ErrorsRoot) -> Self {
        let mut device = std::collections::HashMap::new();
        let mut hms = std::collections::HashMap::new();

        for e in errors.data.device_error.en {
            if let Ok(code) = u64::from_str_radix(&e.ecode, 16) {
                device.insert(code, e.intro);
            } else {
                error!("Failed to parse Device ecode: {}", e.ecode);
            }
        }

        for e in errors.data.device_hms.en {
            if let Ok(code) = u64::from_str_radix(&e.ecode, 16) {
                hms.insert(code, e.intro);
            } else {
                error!("Failed to parse HMS ecode: {}", e.ecode);
            }
        }

        Self { device, hms }
    }
}

/// see also:
/// https://github.com/greghesp/ha-bambulab/blob/main/custom_components/bambu_lab/pybambu/const.py
async fn fetch_error_codes() -> Result<ErrorsRoot> {
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let res = client.get(ERRORS_URL).send().await?;

    if !res.status().is_success() {
        debug!("res {:#?}", res);
        bail!("Failed to get response, url = {}", ERRORS_URL);
    }

    let json: ErrorsRoot = res.json().await?;
    // let json: serde_json::Value = res.json().await?;

    // debug!("json {:#?}", json);

    // let s = serde_json::to_string_pretty(&json)?;

    // /// write s to file
    // let mut file = File::create("errors.json")?;
    // file.write_all(s.as_bytes())?;

    Ok(json)
    // unimplemented!()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorsRoot {
    data: ErrorCodes,
    result: i64,
    t: i64,
    ver: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorCodes {
    device_error: Errors,
    device_hms: Errors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Errors {
    en: Vec<ErrorCode>,
    // en: HashMap<String, String>,
    ver: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorCode {
    ecode: String,
    intro: String,
}
