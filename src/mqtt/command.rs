// TODO: increment sequence command?
#[derive(Debug, Clone)]
pub enum Command {
    /// Get the version of the printer.
    GetVersion,
    /// Pause the current print.
    Pause,
    /// Resume the current print.
    Resume,
    /// Stop the current print.
    Stop,
    /// Get all device information.
    PushAll,
    StartPush,
    SetChamberLight(bool),
    SetSpeedProfile(String),
    SendGCodeTemplate(String),
    GetAccessories,
    ChangeAMSFilamentSetting(ChangeAMSFilamentSetting),
}

#[derive(Debug, Clone)]
pub struct ChangeAMSFilamentSetting {
    ams_id: i64,
    tray_id: i64,
    tray_color: [u8; 3],
    nozzle_temp_min: i64,
    nozzle_temp_max: i64,
    tray_type: String,
}

impl Command {
    pub(crate) fn get_payload(&self) -> String {
        match self {
            Self::GetVersion => GET_VERSION_PAYLOD.into(),
            Self::Pause => PAUSE_PAYLOAD.into(),
            Self::Resume => RESUME_PAYLOAD.into(),
            Self::Stop => STOP_PAYLOAD.into(),
            Self::PushAll => PUSHALL_PAYLOAD.into(),
            Self::StartPush => START_PUSH_PAYLOAD.into(),
            Self::SetChamberLight(on) => {
                SET_CHAMBER_LIGHT_PAYLOAD.replace("<LED_STATUS>", if *on { "on" } else { "off" })
            }
            Self::SetSpeedProfile(profile) => {
                SET_SPEED_PROFILE_PAYLOAD.replace("<PROFILE>", profile)
            }
            Self::SendGCodeTemplate(gcode) => SEND_GCODE_TEMPLATE_PAYLOAD.replace("<GCODE>", gcode),
            Self::GetAccessories => GET_ACCESSORIES_PAYLOAD.into(),
            Self::ChangeAMSFilamentSetting(setting) => {
                // format!(
                //     r#"
                //     "print": {{
                //         "sequence_id": "0",
                //         "command": "ams_filament_setting",
                //         "ams_id": {},
                //         "tray_id": {},
                //         "tray_info_idx": "",
                //         "tray_color": "{:02X}{:02X}{:02X}",
                //         "nozzle_temp_min": {},
                //         "nozzle_temp_max": {},
                //         "tray_type": "{}"
                //     }}
                //     "#,
                //     setting.ams_id,
                //     setting.tray_id,
                //     setting.tray_color[0],
                //     setting.tray_color[1],
                //     setting.tray_color[2],
                //     setting.nozzle_temp_min,
                //     setting.nozzle_temp_max,
                //     setting.tray_type,
                // )
                panic!("ChangeAMSFilamentSetting is not implemented");
            }
        }
    }
}

static GET_VERSION_PAYLOD: &str = r#"{"info": {"sequence_id": "0", "command": "get_version"}}"#;
static PAUSE_PAYLOAD: &str = r#"{"print": {"sequence_id": "0", "command": "pause"}}"#;
static RESUME_PAYLOAD: &str = r#"{"print": {"sequence_id": "0", "command": "resume"}}"#;
static STOP_PAYLOAD: &str = r#"{"print": {"sequence_id": "0", "command": "stop"}}"#;
static PUSHALL_PAYLOAD: &str = r#"{"pushing": {"sequence_id": "0", "command": "pushall"}}"#;
static START_PUSH_PAYLOAD: &str = r#"{"pushing": {"sequence_id": "0", "command": "start"}}"#;
static SET_CHAMBER_LIGHT_PAYLOAD: &str = r#"{"system": {"sequence_id": "0", "command": "ledctrl", "led_node": "chamber_light", "led_mode": "<LED_STATUS>", "led_on_time": 500, "led_off_time": 500, "loop_times": 0, "interval_time": 0}}"#;
static SET_SPEED_PROFILE_PAYLOAD: &str =
    r#"{"print": {"sequence_id": "0", "command": "print_speed", "param": "<PROFILE>"}}"#;
static SEND_GCODE_TEMPLATE_PAYLOAD: &str =
    r#"{"print": {"sequence_id": "0", "command": "gcode_line", "param": "<GCODE>"}}"#;
static GET_ACCESSORIES_PAYLOAD: &str =
    r#"{"system": {"sequence_id": "0", "command": "get_accessories", "accessory_type": "none"}}"#;
