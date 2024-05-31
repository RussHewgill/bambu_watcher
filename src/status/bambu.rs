use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use chrono::{DateTime, Local, TimeDelta};
use egui::Color32;
use std::{
    collections::HashMap,
    option,
    time::{Duration, Instant},
};

use crate::{
    config::PrinterConfig,
    mqtt::message::{PrintAms, PrintData},
    status::PrinterState,
    ui::ui_types::PrintStage,
};

use super::{
    AmsCurrentSlot, AmsSlot, AmsStatus, AmsUnit, PrintError, PrinterStatusExt, PrinterType,
};

#[derive(Default, Debug, Clone)]
pub struct PrinterStatusBambu {
    /// X1, P1, A1, etc
    pub printer_type: Option<PrinterType>,

    pub state: PrinterState,
    // pub stage: Option<PrintStage>,
    pub stage: Option<i64>,
    pub sub_stage: Option<i64>,

    pub stg: Vec<i64>,
    pub stg_cur: i64,

    // pub last_report: Option<PrinterStatusReport>,
    pub last_report: Option<Instant>,

    pub ams: Option<AmsStatus>,
    pub ams_status: Option<i64>,

    pub current_file: Option<String>,
    pub subtask_id: Option<String>,
    pub current_task_thumbnail_url: Option<String>,
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
    pub line_number: Option<i64>,

    pub chamber_light: Option<bool>,

    pub temp_nozzle: Option<f64>,
    pub temp_tgt_nozzle: Option<f64>,
    pub temp_bed: Option<f64>,
    pub temp_tgt_bed: Option<f64>,
    pub temp_chamber: Option<f64>,

    pub fan_gear: Option<i64>,
    pub heatbreak_fan_speed: Option<i64>,
    pub cooling_fan_speed: Option<i64>,
    pub aux_fan_speed: Option<i64>,
    pub chamber_fan_speed: Option<i64>,
}

impl PrinterStatusExt for PrinterStatusBambu {
    type UpdateMsg = PrintData;

    fn state(&self) -> &PrinterState {
        &self.state
    }
    fn update(&mut self, msg: &Self::UpdateMsg) -> Result<()> {
        self.update(msg)
    }
}

impl PrinterStatusBambu {
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
                "FINISH" => Some(PrinterState::Finished),
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

    pub fn get_print_stage(&self) -> PrintStage {
        if self.stage == Some(768) {
            // return PrintStage::
        }
        //
        unimplemented!()
    }
}

impl PrinterStatusBambu {
    pub fn update(&mut self, report: &PrintData) -> Result<()> {
        self.last_report = Some(Instant::now());

        if let Some(f) = report.gcode_file.as_ref() {
            self.current_file = Some(f.clone());
        }

        if let Some(s) = Self::get_state(report) {
            // if self.state != s && s == PrinterState::Finished {
            //     let _ = notify_rust::Notification::new()
            //         .summary(&format!("Print Complete on {}", printer.name))
            //         .body(&format!(
            //             "{}",
            //             self.current_file
            //                 .as_ref()
            //                 .unwrap_or(&"Unknown File".to_string())
            //         ))
            //         // .icon("thunderbird")
            //         .appname("Bambu Watcher")
            //         .timeout(0)
            //         .show();
            // }
            self.state = s;
        }

        if let Some(s) = report.mc_print_stage.as_ref() {
            // self.stage = Some(s.clone());
            if let Some(s) = s.parse::<i64>().ok() {
                self.stage = Some(s);
            } else {
                warn!("Failed to parse stage: {:?}", s);
            }
        }

        if let Some(s) = report.mc_print_sub_stage {
            self.sub_stage = Some(s);
        }

        if let Some(s) = report.stg.as_ref() {
            self.stg = s.clone();
        }
        if let Some(s) = report.stg_cur {
            self.stg_cur = s;
        }

        // if let Some(s) = report.gcode_state.as_ref() {
        //     self.gcode_state = Some(GcodeState::from_str(s));
        // }

        if let Some(id) = report.subtask_id.as_ref() {
            // debug!("printer name = {:?}", printer.name);
            // debug!("subtask_id = {:?}", id);
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

        if let Some(l) = report.mc_print_line_number.as_ref() {
            if let Some(l) = l.parse::<i64>().ok() {
                self.line_number = Some(l);
            }
        }

        if let Some(lights) = report.lights_report.as_ref() {
            for light in lights.iter() {
                if light.node == "chamber_light" {
                    self.chamber_light = Some(light.mode == "on");
                }
            }
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

        if let Some(t) = report.fan_gear {
            self.fan_gear = Some(t);
        }

        if let Some(t) = self.heatbreak_fan_speed.as_ref() {
            let t = (*t as f32 / 1.5).round() as i64 * 10;
            self.heatbreak_fan_speed = Some(t);
        }

        if let Some(t) = report.cooling_fan_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                let t = (t as f32 / 1.5).round() as i64 * 10;
                self.cooling_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan1_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                let t = (t as f32 / 1.5).round() as i64 * 10;
                self.aux_fan_speed = Some(t);
            }
        }

        if let Some(t) = report.big_fan2_speed.as_ref() {
            if let Some(t) = t.parse::<i64>().ok() {
                let t = (t as f32 / 1.5).round() as i64 * 10;
                self.chamber_fan_speed = Some(t);
            }
        }

        if let Some(s) = report.ams_status {
            self.ams_status = Some(s);
        }

        if let Some(ams) = report.ams.as_ref() {
            self.ams = Some(self.update_ams(ams, self.ams_status)?);
        }

        Ok(())
    }

    fn update_ams(&mut self, ams: &PrintAms, status_code: Option<i64>) -> Result<AmsStatus> {
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
            // out.current_tray = None;
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

        if let Some(bits) = ams.ams_exist_bits.as_ref() {
            out.ams_exist_bits = Some(bits.clone());
        }

        if let Some(bits) = ams.tray_exist_bits.as_ref() {
            out.tray_exist_bits = Some(bits.clone());
        }

        if let Some(now) = ams.tray_now.as_ref() {
            out.tray_now = Some(now.clone());
        }
        if let Some(pre) = ams.tray_pre.as_ref() {
            out.tray_pre = Some(pre.clone());
        }
        if let Some(tar) = ams.tray_tar.as_ref() {
            out.tray_tar = Some(tar.clone());
        }

        if let Some(v) = ams.version {
            out.version = Some(v);
        }

        if let Some(status_code) = status_code {
            if status_code == 768 {
                out.state = None;
            } else {
                let state = crate::utils::parse_ams_status(&out, status_code);

                // match state {
                //     crate::ui::ui_types::AmsState::FilamentChange(_) => {
                //         // out.current_tray
                //     },
                //     _ => {}
                // }

                out.state = Some(state);
            }
        }

        Ok(out)
    }
}
