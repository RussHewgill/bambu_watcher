use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub code: Value,
    pub error: Value,
    pub message: String,
    pub projects: Vec<Project>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub content: String,
    pub create_time: String,
    pub model_id: String,
    pub name: String,
    pub project_id: String,
    pub status: String,
    pub update_time: String,
    pub user_id: String,
}
