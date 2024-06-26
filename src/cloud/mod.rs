pub mod cloud_types;
pub mod errors;
pub mod projects;
pub mod streaming;
pub mod rtsp;

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, error, info, trace, warn};

use crate::{auth::Token, conn_manager::PrinterId};

use self::cloud_types::Device;

const BASE: &'static str = "https://api.bambulab.com";

const URL_LIST: &'static str = "/v1/iot-service/api/user/bind";
const URL_PRINT: &'static str = "/v1/iot-service/api/user/print";
const URL_PROJECTS: &'static str = "/v1/iot-service/api/user/project";
const URL_TASK: &'static str = "/v1/iot-service/api/user/task/";
const URL_TASKS_LIST: &'static str = "/v1/user-service/my/tasks";
const URL_PROJECT: &'static str = "/v1/iot-service/api/user/project/";
const URL_MESSAGES: &'static str = "/v1/user-service/my/messages";
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
        // debug!("res {:#?}", res);
        debug!("status {:#?}", res.status());
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

pub async fn get_printer_list(token: &Token) -> Result<Vec<Device>> {
    let json: cloud_types::BindList = get_response(token, URL_LIST).await?;

    Ok(json.devices)
}

pub async fn get_project_list(token: &Token) -> Result<Vec<projects::ProjectInfo>> {
    let json: projects::ProjectsInfo = get_response(token, URL_PROJECTS).await?;
    // debug!("json {:#?}", json);
    Ok(json.projects)
}

pub async fn get_task_list(
    token: &Token,
    device: Option<PrinterId>,
    after: Option<String>,
    limit: Option<i64>,
) -> Result<projects::TasksInfo> {
    // let json = get_response(token, "/v1/user-service/my/tasks").await?;
    // debug!("json {:#?}", json);
    let client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
    let mut req = client
        .get(format!("{}{}", BASE, URL_TASKS_LIST))
        .header("Authorization", &format!("Bearer {}", token.get_token()));

    if let Some(limit) = limit {
        req = req.query(&[("limit", &format!("{}", limit))]);
    }

    let res = req.send().await?;

    if !res.status().is_success() {
        // debug!("res {:#?}", res);
        debug!("status {:#?}", res.status());
        bail!("Failed to get response, url = {}", URL_TASKS_LIST);
    }

    Ok(res.json().await?)
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

pub async fn get_printer_status(token: &Token) -> Result<Value> {
    let json = get_response(token, URL_PRINT).await?;
    // debug!("json {:#?}", json);
    Ok(json)
}

pub async fn get_subtask_info(token: &Token, project_id: &str) -> Result<cloud_types::SubtaskInfo> {
    let url = format!("{}{}", URL_TASK, project_id);

    let json: cloud_types::SubtaskInfo = get_response(token, &url).await?;
    // let json: Value = get_response(token, &url).await?;

    // debug!("json {:#?}", json);

    Ok(json)
    // unimplemented!()
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
