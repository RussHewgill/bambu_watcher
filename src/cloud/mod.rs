pub mod cloud_types;
pub mod errors;
pub mod projects;
pub mod streaming;

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::de::DeserializeOwned;
use tracing::{debug, error, info, trace, warn};

use crate::auth::Token;

const BASE: &'static str = "https://api.bambulab.com";

const URL_LIST: &'static str = "/v1/iot-service/api/user/bind";
const URL_PRINT: &'static str = "/v1/iot-service/api/user/print";
const URL_PROJECTS: &'static str = "/v1/iot-service/api/user/project";
const URL_TASK: &'static str = "/v1/iot-service/api/user/task/";
const URL_PROJECT: &'static str = "/v1/iot-service/api/user/project/";
const URL_TTCODE: &'static str = "/v1/iot-service/api/user/ttcode";

/// GET /v1/iot-service/api/user/bind
///     This lists devices "bound" to the current user. As in, all your devices.
///
/// GET /v1/iot-service/api/user/print
///     This accepts the optional query parameter force, which the slicer always sets to true.
///     The response is the current status of the printer.
///
/// GET /v1/iot-service/api/user/project
///     Queries a list of projects for the current user.
///
/// GET /v1/iot-service/api/user/project/{PROJECT_ID}
///     Gets full details about a single project.
///
/// POST /v1/iot-service/api/user/ttcode
///     Gets the TTCode for the printer. This is used for authentication to the webcam stream.

pub async fn get_response<T: DeserializeOwned>(token: &Token, url: &str) -> Result<T> {
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let res = client
        .get(format!("{}{}", BASE, url))
        .header("Authorization", &format!("Bearer {}", token.get_token()))
        .send()
        .await?;

    if !res.status().is_success() {
        debug!("res {:#?}", res);
        bail!("Failed to get response, url = {}", url);
    }

    Ok(res.json().await?)
}

#[cfg(feature = "nope")]
pub async fn get_response(token: &Token, url: &str) -> Result<serde_json::Value> {
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let res = client
        .get(format!("{}{}", BASE, url))
        .header("Authorization", &format!("Bearer {}", token.get_token()))
        .send()
        .await?;

    if !res.status().is_success() {
        debug!("res {:#?}", res);
        bail!("Failed to get response, url = {}", url);
    }

    let json: serde_json::Value = res.json().await?;

    Ok(json)
}

pub async fn get_project_list(token: &Token) -> Result<Vec<projects::ProjectInfo>> {
    let json = get_response(token, URL_PROJECTS).await?;
    // debug!("json {:#?}", json);
    Ok(json)
}

pub async fn get_project_info(
    token: &Token,
    project_id: &str,
) -> Result<projects::ProjectDataJson> {
    let url = format!("{}{}", URL_PROJECT, project_id);
    let json = get_response(token, &url).await?;
    // debug!("json {:#?}", json);
    Ok(json)
}

pub async fn get_printer_status(token: &Token) -> Result<()> {
    let json = get_response(token, URL_PRINT).await?;
    debug!("json {:#?}", json);
    Ok(())
}

pub async fn get_subtask_info(token: &Token, project_id: &str) -> Result<cloud_types::SubtaskInfo> {
    let url = format!("{}{}", URL_TASK, project_id);

    let json: cloud_types::SubtaskInfo = get_response(token, &url).await?;

    Ok(json)
}

#[cfg(feature = "nope")]
pub fn get_machines_list(token: &Token) -> Result<()> {
    debug!("get_current_thumbnail");
    // let token = Token

    // let mut map = HashMap::new();
    // map.insert("account", username);
    // map.insert("password", pass);
    // map.insert("apiError", "");

    let url = "https://api.bambulab.com/v1/iot-service/api/user/bind";

    let client = reqwest::blocking::ClientBuilder::new()
        .use_rustls_tls()
        .build()?;
    let res = client
        .get(url)
        // .header(&map)
        .header("Authorization", &format!("Bearer {}", token.get_token()))
        .send()?;

    debug!("res {:#?}", res);

    if !res.status().is_success() {
        bail!("Failed to get current thumbnail");
    }

    // debug!("res {:#?}", res);
    let json: serde_json::Value = res.json()?;

    debug!("json {:#?}", json);

    Ok(())
}
