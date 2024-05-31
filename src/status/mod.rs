pub mod bambu;
pub mod klipper;

use anyhow::{anyhow, bail, ensure, Context, Result};
use bambu::PrinterStatusBambu;
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
    ui::ui_types::PrintStage,
};

// use crate::app_types::StatusIcon;

pub trait PrinterStatusExt {
    type UpdateMsg;

    fn state(&self) -> &PrinterState;

    fn update(&mut self, msg: &Self::UpdateMsg) -> Result<()>;

    fn is_error(&self) -> bool {
        matches!(self.state(), &PrinterState::Error(_))
    }
}

#[derive(Debug, Clone, strum::EnumDiscriminants)]
#[strum_discriminants(name(PrinterStatusType))]
pub enum PrinterStatus {
    Bambu(PrinterStatusBambu),
    Klipper(PrinterStatusKlipper),
    Prusa(PrinterStatusPrusa),
}

impl PrinterStatus {
    pub fn empty(printer_type: PrinterStatusType) -> Self {
        match printer_type {
            PrinterStatusType::Bambu => PrinterStatus::Bambu(PrinterStatusBambu::default()),
            PrinterStatusType::Klipper => todo!(),
            PrinterStatusType::Prusa => todo!(),
        }
    }

    pub fn state(&self) -> &PrinterState {
        match self {
            PrinterStatus::Bambu(s) => &s.state,
            PrinterStatus::Klipper(_) => todo!(),
            PrinterStatus::Prusa(_) => todo!(),
        }
    }

    pub fn is_error(&self) -> bool {
        unimplemented!()
    }
}

#[derive(Default, Debug, Clone)]
pub struct PrinterStatusKlipper {
    //
}

#[derive(Default, Debug, Clone)]
pub struct PrinterStatusPrusa {
    //
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrinterType {
    X1C,
    X1E,
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
    pub ams_exist_bits: Option<String>,
    pub tray_exist_bits: Option<String>,
    pub tray_now: Option<String>,
    pub tray_pre: Option<String>,
    pub tray_tar: Option<String>,
    pub version: Option<i64>,
    pub state: Option<crate::ui::ui_types::AmsState>,
}

impl AmsStatus {
    pub fn is_ams_unload(&self) -> bool {
        self.tray_tar.as_ref().map(|s| s.as_str()) == Some("255")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AmsCurrentSlot {
    ExternalSpool,
    Tray { ams_id: u64, tray_id: u64 },
}

impl AmsCurrentSlot {
    pub fn is_slot(&self, ams_id: u64, tray_id: u64) -> bool {
        match self {
            AmsCurrentSlot::Tray {
                ams_id: a,
                tray_id: t,
            } => *a == ams_id && *t == tray_id,
            _ => false,
        }
    }
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
