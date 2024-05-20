use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    mqtt::message::{InfoData, InfoModule},
    status::PrinterType,
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
