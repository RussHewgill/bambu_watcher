use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    mqtt::message::{InfoData, InfoModule},
    status::{AmsStatus, PrinterType},
    ui::ui_types::{AmsState, FilamentSwapStep},
};

/// https://github.com/greghesp/ha-bambulab/blob/main/custom_components/bambu_lab/pybambu/utils.py#L119
pub fn get_printer_type(info: &InfoData) -> PrinterType {
    let Some(ap_node) = info.module.iter().find(|m| m.name == "ap") else {
        return PrinterType::Unknown;
    };

    // debug!("ap_node {:#?}", ap_node);

    let hw_ver = ap_node.hw_ver.as_str();
    let Some(project_name) = ap_node.project_name.as_ref() else {
        return PrinterType::X1C;
    };

    match hw_ver {
        "AP02" => PrinterType::X1E,
        "AP04" => match project_name.as_str() {
            "C11" => PrinterType::P1P,
            "C12" => PrinterType::P1S,
            _ => PrinterType::Unknown,
        },
        "AP05" => match project_name.as_str() {
            "N1" => PrinterType::A1m,
            "N2S" => PrinterType::A1,
            _ => PrinterType::X1C,
        },
        _ => PrinterType::Unknown,
    }
}

/// Orcaslicer: src/slic3r/GUI/StatusPanel.cpp: update_ams
pub fn parse_ams_status(status: &AmsStatus, status_code: i64) -> AmsState {
    let (sub_state, state) = _parse_ams_status(status_code);

    if matches!(state, AmsState::FilamentChange(_)) {
        let step = match sub_state {
            0x02 => FilamentSwapStep::HeatNozzle,
            0x03 => FilamentSwapStep::CutFilament,
            0x04 => FilamentSwapStep::PullBackCurrentFilament,
            0x05 => {
                if !status.is_ams_unload() {
                    /// TODO: if m_is_load_with_temp is set, cut filament
                    FilamentSwapStep::PushNewFilament
                } else {
                    FilamentSwapStep::PushNewFilament
                }
            }
            0x06 => FilamentSwapStep::PushNewFilament,
            0x07 => FilamentSwapStep::PurgeOldFilament,
            0x08 => FilamentSwapStep::CheckFilamentPosition,
            _ => FilamentSwapStep::Idling,
        };

        AmsState::FilamentChange(step)
    } else {
        state
    }
}

fn _parse_ams_status(status_code: i64) -> (i64, AmsState) {
    let ams_status_sub = status_code & 0xFF;
    let ams_status_main_int = (status_code & 0xFF00) >> 8;

    debug!("ams_status_sub: {:#?}", ams_status_sub);
    debug!("ams_status_main_int: {:#?}", ams_status_main_int);

    let state = match ams_status_main_int {
        0 => AmsState::Idle,
        1 => AmsState::FilamentChange(FilamentSwapStep::Idling),
        2 => AmsState::RfidIdentifying,
        3 => AmsState::Assist,
        4 => AmsState::Calibration,
        0x10 => AmsState::SelfCheck,
        0x20 => AmsState::Debug,
        _ => AmsState::Unknown,
    };

    (ams_status_sub, state)
}

// pub fn get_ams_filament_swap_step() -> FilamentSwapStep {
//     unimplemented!()
// }
