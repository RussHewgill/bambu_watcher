use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

pub type PrinterId = Arc<String>;

/// messages from PrinterConnManager to UI
#[derive(Debug)]
pub enum PrinterConnMsg {
    /// The current status of a printer
    StatusReport(PrinterId, PrintData),
    LoggedIn,
    SyncedProjects(crate::ui::ui_types::ProjectsList),
    SyncedPrinters,
}

#[derive(Debug)]
pub enum CloudService {
    Bambu,
}

/// messages from UI to PrinterConnManager
#[derive(Debug)]
pub enum PrinterConnCmd {
    // SyncPrinters,
    // AddPrinter(PrinterConfig),
    // RemovePrinter(PrinterId),
    // UpdatePrinterConfig(PrinterId, NewPrinterEntry),
    // SetPrinterCloud(PrinterId, bool),

    // SyncProjects,
    /// get the status of a printer
    ReportStatus(PrinterId),
    ReportInfo(PrinterId),

    Login(CloudService, String, String),
    Logout,
}

pub struct PrinterConnManager {}
