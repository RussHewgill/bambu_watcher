use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use bambulab::Message;
use tracing::{debug, error, info, trace, warn};

use tokio::sync::broadcast::{Receiver, Sender};

use crate::config::{Configs, PrinterConfig};

/// The serial number of a printer
pub type PrinterId = String;

#[derive(Debug, Clone)]
pub enum PrinterStatus {
    Idle,
    Printing(Duration),
    Error(String),
}

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    /// The current status of a printer
    StatusReport(PrinterId, PrinterStatus),
}

/// messages from UI to PrinterConnManager
#[derive(Debug, Clone)]
pub enum PrinterConnCmd {
    /// get the status of a printer
    ReportStatus(PrinterId),
}

pub struct PrinterConnManager {
    config: Configs,
    // printer_rx: HashMap<PrinterId, Receiver<Message>>,
    cmd_rx: Receiver<PrinterConnCmd>,
    msg_tx: Sender<PrinterConnMsg>,
}

impl PrinterConnManager {
    pub fn new(config: Configs, cmd_rx: Receiver<PrinterConnCmd>, msg_tx: Sender<PrinterConnMsg>) -> Self {
        Self {
            config,
            // printer_rx: HashMap::new(),
            cmd_rx,
            msg_tx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<(PrinterId, Message)>(25);

        for printer in self.config.printers.iter() {
            Self::start_printer_listener(tx.clone(), printer).await?;
        }

        loop {
            tokio::select! {
                Ok(cmd) = self.cmd_rx.recv() => {
                    debug!("got cmd = {:?}", cmd);
                }
                Ok(printer_msg) = rx.recv() => {
                    debug!("got printer_msg, id = {:?} = {:?}", printer_msg.0, printer_msg.1);
                }
            }
            // break;
        }

        // Ok(())
    }

    async fn start_printer_listener(
        // tx: tokio::sync::broadcast::Sender<Message>,
        msg_tx: tokio::sync::broadcast::Sender<(PrinterId, Message)>,
        printer: &PrinterConfig,
    ) -> Result<()> {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Message>(25);
        let mut client = bambulab::Client::new(&printer.host, &printer.access_code, &printer.serial, tx);
        tokio::spawn(async move {
            client.run().await.unwrap();
        });
        let serial = printer.serial.clone();
        tokio::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();
                msg_tx.send((serial.clone(), message)).unwrap();
            }
        });
        Ok(())
    }
}

#[cfg(feature = "nope")]
mod old {

    pub struct PrinterConnManager {
        // pub printers: HashMap<String, PrinterConn>,
        pub printers_chans: HashMap<String, Sender<PrinterConnCmd>>,
        pub cmd_rx: Receiver<PrinterConnCmd>,
    }

    pub struct PrinterConn {
        msg_rx: tokio::sync::broadcast::Receiver<Message>,
    }

    impl PrinterConnManager {
        pub fn new() -> Self {
            Self { printers: vec![] }
        }

        pub async fn run(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[cfg(feature = "nope")]
    async fn start_printer_listener(
        tx: tokio::sync::broadcast::Sender<Message>,
        host: &str,
        access_code: &str,
        serial: &str,
    ) -> Result<()> {
        let mut client = bambulab::Client::new(host, access_code, serial, tx);
        client.run().await.unwrap();
        Ok(())
    }
}
