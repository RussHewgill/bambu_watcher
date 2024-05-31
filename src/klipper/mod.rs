pub mod klipper_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::de::DeserializeOwned;

/// GET
const URL_SERVER_INFO: &'static str = "/server/info";
const URL_PRINTER_INFO: &'static str = "/printer/info";
const URL_OBJECTS_LIST: &'static str = "/printer/objects/list";
const URL_WEBCAM_LIST: &'static str = "/server/webcam/list";

/// POST
const URL_EMERGENCY_STOP: &'static str = "/printer/emergency_stop";
const URL_PRINTER_RESTART: &'static str = "/printer/restart";
const URL_FIRMWARE_RESTART: &'static str = "/printer/firmware_restart";
const URL_PAUSE_PRINT: &'static str = "/printer/print/pause";
const URL_RESUME_PRINT: &'static str = "/printer/print/resume";
const URL_RESUME_CANCEL: &'static str = "/printer/print/cancel";

pub async fn get_response<T: DeserializeOwned>(host: &str, url: &str) -> Result<T> {
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let res = client
        .get(format!("http://{}{}", host, url))
        // .get(url)
        // .header("Authorization", &format!("Bearer {}", token.get_token()))
        .send()
        .await?;

    if !res.status().is_success() {
        // debug!("res {:#?}", res);
        debug!("status {:#?}", res.status());
        bail!("Failed to get response, url = {}", url);
    }

    Ok(res.json().await?)
}
