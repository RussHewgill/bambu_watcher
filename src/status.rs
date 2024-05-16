use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use chrono::{DateTime, Local, TimeDelta};
use egui::Color32;

// use bambulab::PrintData;
use crate::{
    config::PrinterConfig,
    mqtt::message::{PrintAms, PrintData},
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

// use crate::app_types::StatusIcon;

#[derive(Default, Debug, Clone)]
pub struct PrinterStatus {
    /// X1, P1, A1, etc
    pub printer_type: Option<PrinterType>,

    pub state: PrinterState,
    // pub last_report: Option<PrinterStatusReport>,
    pub last_report: Option<Instant>,

    pub ams: Option<AmsStatus>,

    pub current_file: Option<String>,
    pub subtask_id: Option<String>,
    // pub gcode_state: Option<GcodeState>,
    pub print_error: Option<PrintError>,
    pub print_percent: Option<i64>,
    pub eta: Option<DateTime<Local>>,
    pub is_sdcard_printing: Option<bool>,

    pub wifi_signal: Option<String>,
    pub spd_lvl: Option<i64>,
    // pub print_line_number: Option<String>,
    pub layer_num: Option<i64>,
    pub total_layer_num: Option<i64>,

    pub temp_nozzle: Option<f64>,
    pub temp_tgt_nozzle: Option<f64>,
    pub temp_bed: Option<f64>,
    pub temp_tgt_bed: Option<f64>,
    pub temp_chamber: Option<f64>,

    pub heatbreak_fan_speed: Option<i64>,
    pub cooling_fan_speed: Option<i64>,
    pub aux_fan_speed: Option<i64>,
    pub chamber_fan_speed: Option<i64>,
}

impl PrinterStatus {
    pub fn is_error(&self) -> bool {
        matches!(self.state, PrinterState::Error(_))
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn get_state(report: &PrintData) -> Option<PrinterState> {
        if let Some(s) = report.gcode_state.as_ref() {
            match s.as_str() {
                "IDLE" => Some(PrinterState::Idle),
                "READY" => Some(PrinterState::Idle),
                "FINISH" => Some(PrinterState::Idle),
                "CREATED" => Some(PrinterState::Printing),
                "RUNNING" => Some(PrinterState::Printing),
                "PREPARE" => Some(PrinterState::Printing),
                "PAUSE" => {
                    if let Some(e) = report.print_error {
                        Some(PrinterState::Error(format!("Error: {}", e)))
                    } else {
                        Some(PrinterState::Paused)
                    }
                }
                "FAILED" => Some(PrinterState::Error("Failed".to_string())),
                // s => panic!("Unknown gcode state: {}", s),
                s => Some(PrinterState::Unknown(s.to_string())),
            }
        } else {
            None
        }
    }

    pub fn update(&mut self, printer: &PrinterConfig, report: &PrintData) -> Result<()> {
        self.last_report = Some(Instant::now());

        if let Some(f) = report.gcode_file.as_ref() {
            self.current_file = Some(f.clone());
        }

        if let Some(s) = Self::get_state(report) {
            if self.state != s && s == PrinterState::Finished {
                let _ = notify_rust::Notification::new()
                    .summary(&format!("Print Complete on {}", printer.name))
                    .body(&format!(
                        "{}",
                        self.current_file
                            .as_ref()
                            .unwrap_or(&"Unknown File".to_string())
                    ))
                    // .icon("thunderbird")
                    .appname("Bambu Watcher")
                    .timeout(0)
                    .show();
            }
            self.state = s;
        }

        // if let Some(s) = report.gcode_state.as_ref() {
        //     self.gcode_state = Some(GcodeState::from_str(s));
        // }

        if let Some(id) = report.subtask_id.as_ref() {
            debug!("printer name = {:?}", printer.name);
            debug!("subtask_id = {:?}", id);
            self.subtask_id = Some(id.clone());
        }

        if let Some(p) = report.mc_percent {
            self.print_percent = Some(p);
        }

        if let Some(e) = report.print_error {
            self.print_error = Some(PrintError::from_code(e));
        }

        if let Some(t) = report.mc_remaining_time {
            self.eta = Some(
                Local::now()
                    + TimeDelta::new(t as i64 * 60, 0).context(format!("time delta: {:?}", t))?,
            );
        }

        if let Some(w) = report.wifi_signal.as_ref() {
            self.wifi_signal = Some(w.clone());
        }

        if let Some(s) = report.spd_lvl {
            self.spd_lvl = Some(s);
        }

        if let Some(l) = report.layer_num {
            self.layer_num = Some(l);
        }

        if let Some(t) = report.total_layer_num {
            self.total_layer_num = Some(t);
        }

        if let Some(t) = report.nozzle_temper {
            self.temp_nozzle = Some(t);
        }
        if let Some(t) = report.nozzle_target_temper {
            self.temp_tgt_nozzle = Some(t as f64);
        }

        if let Some(t) = report.bed_temper {
            self.temp_bed = Some(t);
        }
        if let Some(t) = report.bed_target_temper {
            self.temp_tgt_bed = Some(t as f64);
        }

        if let Some(t) = report.chamber_temper {
            self.temp_chamber = Some(t);
        }

        // if let Some(t) = report.heatbreak_fan_speed {
        //     self.heatbreak_fan_speed = Some(t);
        // }

        if let Some(t) = report.cooling_fan_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                self.cooling_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan1_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                self.aux_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan2_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                self.chamber_fan_speed = Some(t);
            }
        }

        if let Some(ams) = report.ams.as_ref() {
            self.ams = Some(self.update_ams(ams)?);
        }

        Ok(())
    }

    fn update_ams(&mut self, ams: &PrintAms) -> Result<AmsStatus> {
        let mut out = self.ams.take().unwrap_or_default();

        // debug!("ams = {:#?}", ams);

        /// 254 if external spool / vt_tray,
        /// otherwise is ((ams_id * 4) + tray_id) for current tray
        /// (ams 2 tray 2 would be (1*4)+1 = 5)
        if let Some(current) = ams.tray_now.as_ref().and_then(|t| t.parse::<u64>().ok()) {
            out.current_tray = if current == 254 {
                Some(AmsCurrentSlot::ExternalSpool)
            } else {
                Some(AmsCurrentSlot::Tray {
                    ams_id: current / 4,
                    tray_id: current % 4,
                })
            };
        } else {
            out.current_tray = None;
        }

        if let Some(units) = ams.ams.as_ref() {
            for unit in units.iter() {
                let mut slots: [Option<AmsSlot>; 4] = Default::default();

                for i in 0..4 {
                    let slot = &unit.tray[i];

                    let Some(col) = slot.tray_color.clone() else {
                        slots[i] = None;
                        continue;
                    };
                    let color = Color32::from_hex(&format!("#{}", col))
                        .unwrap_or(Color32::from_rgb(255, 0, 255));

                    slots[i] = Some(AmsSlot {
                        material: slot.tray_type.clone().unwrap_or("Unknown".to_string()),
                        k: slot.k.unwrap_or(0.),
                        color,
                    });
                }

                let id = unit.id.parse::<i64>()?;

                // out.units.push(AmsUnit {
                //     id,
                //     humidity: unit.humidity.parse().unwrap_or(0),
                //     temp: unit.temp.parse().unwrap_or(0.),
                //     slots,
                // });
                out.units.insert(
                    id,
                    AmsUnit {
                        id,
                        humidity: unit.humidity.parse().unwrap_or(0),
                        temp: unit.temp.parse().unwrap_or(0.),
                        slots,
                    },
                );
            }
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrinterType {
    X1,
    P1P,
    P1S,
    A1,
    A1m,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrinterState {
    Idle,
    Finished,
    Paused,
    Printing,
    Error(String),
    Disconnected,
    Unknown(String),
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
            PrinterState::Finished => "Finished",
            PrinterState::Printing => "Printing",
            PrinterState::Error(_) => "Error",
            PrinterState::Paused => "Paused",
            PrinterState::Disconnected => "Disconnected",
            PrinterState::Unknown(s) => "Unknown",
        }
    }

    // pub fn to_char(&self) -> &'static str {
    //     match self {
    //         PrinterState::Idle => "💤",
    //         PrinterState::Printing => "🟢",
    //         PrinterState::Error(_) => "🟥️",
    //         PrinterState::Paused => "🟡",
    //         PrinterState::Disconnected => "🔌",
    //     }
    // }

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

/// check available actions
impl PrinterState {
    pub fn can_print(&self) -> bool {
        !matches!(self, PrinterState::Printing)
    }
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

/// https://e.bambulab.com/query.php?
impl PrintError {
    pub fn from_code(code: i64) -> Self {
        match code {
            0 => PrintError::None,
            // 83935249 => PrintError::None,
            _ => PrintError::Unknown(code),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AmsStatus {
    // pub units: Vec<AmsUnit>,
    pub units: HashMap<i64, AmsUnit>,
    pub current_tray: Option<AmsCurrentSlot>,
    // pub id: Option<i64>,
    // pub humidity: Option<i64>,
    // pub temp: Option<i64>,
    // pub slots: [Option<AmsSlot>; 4],
    // pub current_slot: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum AmsCurrentSlot {
    ExternalSpool,
    Tray { ams_id: u64, tray_id: u64 },
}

#[derive(Debug, Default, Clone)]
pub struct AmsUnit {
    pub id: i64,
    pub humidity: i64,
    pub temp: f64,
    pub slots: [Option<AmsSlot>; 4],
}

#[derive(Debug, Default, Clone)]
pub struct AmsSlot {
    pub material: String,
    pub k: f64,
    // pub color: [u8; 3],
    pub color: egui::Color32,
}
