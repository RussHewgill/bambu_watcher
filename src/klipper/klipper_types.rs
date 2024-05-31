use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KlipperInfo {
    pub state: String,
    pub state_message: String,
    pub hostname: String,
    pub software_version: String,
    pub cpu_info: String,
    pub klipper_path: String,
    pub python_path: String,
    pub log_file: String,
    pub config_file: String,
}
