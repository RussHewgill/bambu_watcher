use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use self::project_data_json::ProjectDataJson;
pub use self::{project_data::ProjectData, task_data::TaskData};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectsInfo {
    pub code: Value,
    pub error: Value,
    pub message: String,
    pub projects: Vec<ProjectInfo>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub content: String,
    pub create_time: String,
    pub model_id: String,
    pub name: String,
    pub project_id: String,
    pub status: String,
    pub update_time: String,
    pub user_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TasksInfo {
    pub hits: Vec<TaskInfoJson>,
    pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInfoJson {
    #[serde(rename = "amsDetailMapping")]
    pub ams_detail_mapping: Vec<AmsDetailMapping>,
    #[serde(rename = "bedType")]
    pub bed_type: String,
    #[serde(rename = "costTime")]
    pub cost_time: i64,
    pub cover: String,
    #[serde(rename = "designId")]
    pub design_id: i64,
    #[serde(rename = "designTitle")]
    pub design_title: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "deviceModel")]
    pub device_model: String,
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    #[serde(rename = "feedbackStatus")]
    pub feedback_status: i64,
    pub id: i64,
    #[serde(rename = "instanceId")]
    pub instance_id: i64,
    #[serde(rename = "isPrintable")]
    pub is_printable: bool,
    #[serde(rename = "isPublicProfile")]
    pub is_public_profile: bool,
    pub length: i64,
    pub mode: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(rename = "plateIndex")]
    pub plate_index: i64,
    #[serde(rename = "plateName")]
    pub plate_name: String,
    #[serde(rename = "profileId")]
    pub profile_id: i64,
    #[serde(rename = "startTime")]
    pub start_time: String,
    pub status: i64,
    pub title: String,
    pub weight: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmsDetailMapping {
    pub ams: i64,
    #[serde(rename = "filamentId")]
    pub filament_id: String,
    #[serde(rename = "filamentType")]
    pub filament_type: String,
    #[serde(rename = "sourceColor")]
    pub source_color: String,
    #[serde(rename = "targetColor")]
    pub target_color: String,
    #[serde(rename = "targetFilamentType")]
    pub target_filament_type: String,
    pub weight: f64,
}

impl project_data::ProjectData {
    #[cfg(feature = "nope")]
    pub fn from_json(json: Value) -> Result<Self> {
        let profile = json.get("profiles").context("")?.get(0).context("")?;

        let mut materials = vec![];
        for mat in profile
            .get("materials")
            .context("")?
            .as_array()
            .context("")?
        {
            let color =
                egui::Color32::from_hex(mat.get("color").context("")?.as_str().context("")?)
                    .unwrap_or_default();
            let m = mat
                .get("material")
                .context("")?
                .as_str()
                .context("")?
                .to_string();
            materials.push(([color.r(), color.g(), color.b()], m));
        }

        Ok(Self {
            name: json
                .get("name")
                .context("")?
                .as_str()
                .context("")?
                .to_string(),
            project_id: json
                .get("project_id")
                .context("")?
                .as_str()
                .context("")?
                .to_string(),
            profile_id: profile
                .get("profile_id")
                .context("")?
                .as_str()
                .context("")?
                .to_string(),
            create_time: chrono::DateTime::parse_from_rfc3339(
                &json.get("create_time").context("")?.as_str().context("")?,
            )
            .unwrap()
            .with_timezone(&chrono::Utc),
            materials,
        })
    }

    // #[cfg(feature = "nope")]
    pub fn from_json(json: ProjectDataJson) -> Result<Self> {
        let mut materials = Vec::new();
        for material in json.profiles[0].context.materials.iter() {
            let c = egui::Color32::from_hex(&material.color).unwrap_or_default();
            materials.push(([c.r(), c.g(), c.b()], material.material.clone()));
        }

        let create_time =
            chrono::NaiveDateTime::parse_from_str(&json.create_time, "%Y-%m-%d %H:%M:%S")?
                .and_utc();

        #[cfg(feature = "nope")]
        for plate in json.profiles[0].context.plates.iter() {
            let mut filaments = vec![];

            if let Some(filaments_json) = plate.filaments.as_ref() {
                for filament in filaments_json.iter() {
                    let c = egui::Color32::from_hex(&filament.color).unwrap_or_default();
                    filaments.push(project_data::Filament {
                        color: [c.r(), c.g(), c.b()],
                        id: filament.id.parse()?,
                        type_field: filament.type_field.clone(),
                        used_g: filament.used_g.parse()?,
                        used_m: filament.used_m.parse()?,
                    });
                }
            }

            plates.push(project_data::Plate {
                index: plate.index,
                pick_picture: plate.pick_picture.clone(),
                top_picture: plate.top_picture.clone(),
                thumbnail: plate.thumbnail.clone(),
                weight: plate.weight.unwrap_or_default(),
                time: plate.prediction.unwrap_or_default(),
                filaments,
            });
        }

        let mut content = vec![];
        let c: Value = serde_json::from_str(&json.content)?;
        for plate in c
            .get("printed_plates")
            .context("")?
            .as_array()
            .context("")?
        {
            let plate = plate.get("plate").context("")?;
            content.push(plate.as_i64().unwrap());
        }

        let plate_id = *content.get(0).context("no plates")?;

        let plate = {
            let plate = json.profiles[0]
                .context
                .plates
                .iter()
                .find(|p| p.index == plate_id)
                .context("no plate")?;

            let mut filaments = vec![];

            if let Some(filaments_json) = plate.filaments.as_ref() {
                for filament in filaments_json.iter() {
                    let c = egui::Color32::from_hex(&filament.color).unwrap_or_default();
                    filaments.push(project_data::Filament {
                        color: [c.r(), c.g(), c.b()],
                        id: filament.id.parse()?,
                        type_field: filament.type_field.clone(),
                        used_g: filament.used_g.parse()?,
                        used_m: filament.used_m.parse()?,
                    });
                }
            }

            project_data::Plate {
                index: plate.index,
                pick_picture: plate.pick_picture.clone(),
                top_picture: plate.top_picture.clone(),
                thumbnail: plate.thumbnail.clone(),
                weight: plate.weight.unwrap_or_default(),
                time: plate.prediction.unwrap_or_default(),
                filaments,
            }
        };

        Ok(Self {
            name: json.name,
            // content,
            status: json.status.clone(),
            project_id: json.project_id,
            profile_id: json.profiles[0].profile_id.clone(),
            create_time,
            materials,
            plate,
        })
    }
}

pub mod task_data {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct TaskData {
        pub title: String,
        pub status: i64,
        pub weight: f64,
        pub bed_type: String,
        pub cover: String,
        pub device_id: String,
        pub device_model: String,
        pub device_name: String,
        pub start_time: DateTime<Utc>,
        pub end_time: DateTime<Utc>,
        pub id: i64,
        pub length: i64,
        pub cost_time: i64,
        pub plate_index: i64,
    }

    impl TaskData {
        pub fn from_json(json: &super::TaskInfoJson) -> Self {
            Self {
                title: json.title.clone(),
                status: json.status,
                weight: json.weight,
                bed_type: json.bed_type.clone(),
                cover: json.cover.clone(),
                device_id: json.device_id.clone(),
                device_model: json.device_model.clone(),
                device_name: json.device_name.clone(),
                start_time: DateTime::parse_from_rfc3339(&json.start_time)
                    .unwrap()
                    .to_utc(),
                end_time: DateTime::parse_from_rfc3339(&json.end_time)
                    .unwrap()
                    .to_utc(),
                id: json.id,
                length: json.length,
                cost_time: json.cost_time,
                plate_index: json.plate_index,
            }
        }
    }
}

pub mod project_data {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    use super::project_data_json::Thumbnail;

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ProjectData {
        pub name: String,
        pub status: String,
        pub project_id: String,
        pub profile_id: String,
        pub create_time: DateTime<Utc>,
        // pub print_time: i64,
        pub materials: Vec<([u8; 3], String)>,
        pub plate: Plate,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Plate {
        pub index: i64,
        pub pick_picture: Thumbnail,
        pub thumbnail: Thumbnail,
        pub top_picture: Thumbnail,
        pub weight: f64,
        pub time: i64,
        // pub objects:
        pub filaments: Vec<Filament>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Filament {
        pub color: [u8; 3],
        pub id: i64,
        pub type_field: String,
        pub used_g: f64,
        pub used_m: f64,
    }

    //
}

pub mod project_data_json {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ProjectDataJson {
        pub code: Value,
        pub content: String,
        pub create_time: String,
        pub download_md5: Value,
        pub download_url: Value,
        pub error: Value,
        pub keystore_xml: Value,
        pub message: String,
        pub model_id: String,
        pub name: String,
        pub profiles: Vec<Profile>,
        pub project_id: String,
        pub status: String,
        pub update_time: String,
        pub upload_ticket: Value,
        pub upload_url: Value,
        pub user_id: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Profile {
        pub content: String,
        pub context: Context,
        pub create_time: String,
        pub model_id: String,
        pub name: String,
        pub profile_id: String,
        pub status: String,
        pub update_time: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Context {
        pub auxiliary_bom: Vec<Value>,
        pub auxiliary_guide: Vec<Value>,
        pub auxiliary_other: Vec<Value>,
        pub auxiliary_pictures: Vec<Value>,
        pub compatibility: Compatibility,
        pub configs: Vec<Config>,
        pub materials: Vec<Material>,
        pub other_compatibility: Vec<OtherCompatibility>,
        pub pictures: Value,
        pub plates: Vec<Plate>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Compatibility {
        pub dev_model_name: String,
        pub dev_product_name: String,
        pub nozzle_diameter: f64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Config {
        pub dir: String,
        pub name: String,
        pub url: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Material {
        pub color: String,
        pub material: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct OtherCompatibility {
        pub dev_model_name: String,
        pub dev_product_name: String,
        pub nozzle_diameter: f64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Plate {
        pub filaments: Option<Vec<Filament>>,
        pub gcode: Option<Gcode>,
        pub index: i64,
        pub label_object_enabled: bool,
        pub name: String,
        pub objects: Vec<Object>,
        pub pick_picture: Thumbnail,
        pub prediction: Option<i64>,
        pub skipped_objects: Value,
        pub thumbnail: Thumbnail,
        pub top_picture: Thumbnail,
        #[serde(default, deserialize_with = "default_on_null")]
        pub warning: Vec<Warning>,
        pub weight: Option<f64>,
    }

    fn default_on_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: Deserialize<'de> + Default,
    {
        let opt = Option::deserialize(deserializer)?;
        Ok(opt.unwrap_or_default())
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Filament {
        pub color: String,
        pub id: String,
        #[serde(rename = "type")]
        pub type_field: String,
        pub used_g: String,
        pub used_m: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Gcode {
        pub dir: Value,
        pub name: Value,
        pub url: Value,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Object {
        pub identify_id: String,
        pub name: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Thumbnail {
        pub dir: String,
        pub name: String,
        pub url: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Warning {
        pub error_code: String,
        pub level: String,
        pub msg: String,
    }
}
