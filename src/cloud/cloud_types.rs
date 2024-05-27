use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct BindList {
    pub code: Value,
    pub error: Value,
    pub message: String,
    pub devices: Vec<Device>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub dev_access_code: String,
    pub dev_id: String,
    pub dev_model_name: String,
    pub dev_product_name: String,
    pub name: String,
    pub nozzle_diameter: f64,
    pub online: bool,
    pub print_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubtaskInfo {
    pub code: Option<String>,
    pub content: String,
    pub context: Context,
    pub create_time: String,
    pub error: Option<String>,
    pub job_id: u64,
    pub message: String,
    pub model_id: String,
    pub name: String,
    pub parent: u64,
    pub profile_id: String,
    pub project_id: String,
    pub status: String,
    pub sub_task: Vec<Value>,
    pub subtask: Vec<Value>,
    pub update_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    pub auxiliary_bom: Vec<String>,
    pub auxiliary_guide: Vec<String>,
    pub auxiliary_other: Vec<String>,
    pub auxiliary_pictures: Vec<String>,
    pub compatibility: Compatibility,
    pub configs: Vec<Config>,
    pub materials: Vec<Material>,
    pub other_compatibility: Vec<OtherCompatibility>,
    pub pictures: Option<String>,
    pub plates: Vec<Plate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Compatibility {
    pub dev_model_name: String,
    pub dev_product_name: String,
    pub nozzle_diameter: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub dir: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Material {
    pub color: String,
    pub material: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OtherCompatibility {
    pub dev_model_name: String,
    pub dev_product_name: String,
    pub nozzle_diameter: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Plate {
    pub filaments: Vec<Filament>,
    pub gcode: Gcode,
    pub index: u64,
    pub label_object_enabled: bool,
    pub name: String,
    pub objects: Vec<Object>,
    pub pick_picture: Picture,
    pub prediction: u64,
    pub skipped_objects: Option<String>,
    pub thumbnail: Picture,
    pub top_picture: Picture,
    pub warning: Vec<Warning>,
    pub weight: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Filament {
    pub color: String,
    pub id: String,
    #[serde(rename = "type")]
    pub filament_type: String,
    pub used_g: String,
    pub used_m: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gcode {
    pub dir: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    pub identify_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Picture {
    pub dir: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Warning {
    pub error_code: String,
    pub level: String,
    pub msg: String,
}
