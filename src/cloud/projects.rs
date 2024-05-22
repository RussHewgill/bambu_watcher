use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use self::project_data::ProjectData;
pub use self::project_data_json::ProjectDataJson;

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

        Ok(Self {
            name: json.name,
            project_id: json.project_id,
            profile_id: json.profiles[0].profile_id.clone(),
            create_time,
            materials,
        })
    }
}

pub mod project_data {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ProjectData {
        pub name: String,
        pub project_id: String,
        pub profile_id: String,
        pub create_time: DateTime<Utc>,
        pub materials: Vec<([u8; 3], String)>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Plate {
        pub index: i64,
        pub pick_picture: Picture,
        pub thumbnail: Picture,
        pub top_picture: Picture,
        pub weight: f64,
        // pub objects:
        // pub filaments:
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Picture {
        pub dir: String,
        pub name: String,
        pub url: String,
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
        pub pick_picture: PickPicture,
        pub prediction: Option<i64>,
        pub skipped_objects: Value,
        pub thumbnail: Thumbnail,
        pub top_picture: TopPicture,
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
    pub struct PickPicture {
        pub dir: String,
        pub name: String,
        pub url: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Thumbnail {
        pub dir: String,
        pub name: String,
        pub url: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct TopPicture {
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
