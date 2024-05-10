use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use bambulab::PrintData;
use std::time::{Duration, Instant};

// use crate::app_types::StatusIcon;

#[derive(Default, Debug, Clone)]
pub struct PrinterStatus {
    pub state: PrinterState,
    // pub last_report: Option<PrinterStatusReport>,
    pub last_report: Option<Instant>,
    pub eta: Option<Instant>,

    pub current_file: Option<String>,
    pub gcode_state: Option<String>,
    pub print_percent: Option<i64>,
    pub print_error: Option<PrintError>,
    pub wifi_signal: Option<String>,
    pub spd_lvl: Option<i64>,
    // pub print_line_number: Option<String>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
}

impl PrinterStatus {
    pub fn update(&mut self, report: &bambulab::PrintData) {
        self.last_report = Some(Instant::now());
        // self.eta = report.mc_remaining_time.map(|t| Instant::now() + t);

        // if let Some(f) = report.gcode_file.as_ref() {
        //     self.current_file = Some(f.clone());
        // }

        // self.status = report.status.clone();
        // self.last_report = Some(report);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrinterState {
    Idle,
    Paused,
    Printing(Instant),
    Error(String),
    Disconnected,
}

impl Default for PrinterState {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl PrinterState {
    pub fn to_text(&self) -> &'static str {
        match self {
            PrinterState::Idle => "Idle",
            PrinterState::Printing(_) => "Printing",
            PrinterState::Error(_) => "Error",
            PrinterState::Paused => "Paused",
            PrinterState::Disconnected => "Disconnected",
        }
    }

    pub fn to_char(&self) -> &'static str {
        match self {
            PrinterState::Idle => "💤",
            PrinterState::Printing(_) => "🟢",
            PrinterState::Error(_) => "🟥️",
            PrinterState::Paused => "🟡",
            PrinterState::Disconnected => "🔌",
        }
    }

    // pub fn to_icon(&self) -> StatusIcon {
    //     match self {
    //         PrinterState::Idle => StatusIcon::Idle,
    //         PrinterState::Printing(_) => StatusIcon::PrintingNormally,
    //         PrinterState::Error(_) => StatusIcon::PrintingError,
    //         PrinterState::Paused => StatusIcon::PrintingNormally,
    //         PrinterState::Disconnected => StatusIcon::Disconnected,
    //     }
    // }
}

#[cfg(feature = "nope")]
#[derive(Debug, Clone)]
pub struct PrinterStatusReport {
    pub status: PrinterState,
    pub nozzle_temper: Option<f64>,
    pub nozzle_target_temper: Option<i64>,
    pub bed_temper: Option<f64>,
    pub bed_target_temper: Option<i64>,
    /// print percentage complete
    pub mc_percent: Option<i64>,
    /// in minutes
    pub mc_remaining_time: Option<Duration>,
    pub print_error: Option<PrintError>,
    pub print_type: Option<String>,
    // pub subtask_id: Option<String>,
    pub subtask_name: Option<String>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,
    pub heatbreak_fan_speed: Option<String>,
    pub cooling_fan_speed: Option<String>,
    pub aux_fan_speed: Option<String>,
    pub chamber_fan_speed: Option<String>,
}

#[cfg(feature = "nope")]
impl PrinterStatusReport {
    pub fn from_print_data(i: &PrintData) -> Self {
        // let time_left = Duration::from_secs(i.mc_remaining_time.unwrap() as u64 * 60);
        let time_left = i
            .mc_remaining_time
            .map(|t| Duration::from_secs(t as u64 * 60));
        Self {
            status: PrinterState::Idle,
            // status: match i.status.as_str() {
            //     "Idle" => PrinterStatus::Idle,
            //     // "Printing" => PrinterStatus::Printing(Duration::from_mins(i.mc_remaining_time.unwrap()),
            //     "Error" => PrinterStatus::Error(i.error.clone().unwrap_or_default()),
            //     _ => unreachable!(),
            // },
            nozzle_temper: i.nozzle_temper,
            nozzle_target_temper: i.nozzle_target_temper,
            bed_temper: i.bed_temper,
            bed_target_temper: i.bed_target_temper,
            mc_percent: i.mc_percent,
            mc_remaining_time: time_left,
            print_error: i.print_error.map(PrintError::from_code),
            print_type: i.print_type.clone(),
            // subtask_id: i.subtask_id.clone(),
            subtask_name: i.subtask_name.clone(),
            layer_num: i.layer_num,
            total_layer_num: i.total_layer_num,
            heatbreak_fan_speed: i.heatbreak_fan_speed.clone(),
            cooling_fan_speed: i.cooling_fan_speed.clone(),
            aux_fan_speed: i.big_fan1_speed.clone(),
            chamber_fan_speed: i.big_fan2_speed.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrintError {
    None,
    Unknown(i64),
}

impl PrintError {
    pub fn from_code(code: i64) -> Self {
        match code {
            83935249 => PrintError::None,
            _ => PrintError::Unknown(code),
        }
    }
}
