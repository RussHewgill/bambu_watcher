use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rumqttc::tokio_rustls::{self, rustls};
use std::{collections::HashMap, sync::Arc};
use tokio::{io::AsyncReadExt, sync::RwLock};

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::PrinterId,
};

#[derive(Debug, Clone)]
pub enum StreamCmd {
    StartStream(PrinterId),
    StopStream(PrinterId),
}

pub struct StreamManager {
    configs: ConfigArc,
    streams: HashMap<PrinterId, JpegStreamViewer>,
    handles: HashMap<PrinterId, egui::TextureHandle>,
    kill_tx: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
}

impl StreamManager {
    pub fn new(
        configs: ConfigArc,
        // configs: ConfigArc,
        handles: HashMap<PrinterId, egui::TextureHandle>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
    ) -> Self {
        Self {
            configs,
            streams: HashMap::new(),
            handles,
            kill_tx: HashMap::new(),
            cmd_rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        /// spawn worker tasks
        for id in self.configs.printer_ids_async().await {
            let handle = self.handles.get(&id).unwrap().clone();
            let configs2 = self.configs.clone();

            let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
            self.kill_tx.insert(id.clone(), kill_tx);

            tokio::task::spawn(async move {
                if let Ok(mut streamer) = JpegStreamViewer::new(configs2, id, handle, kill_rx).await
                {
                    if let Err(e) = streamer.run().await {
                        error!("streamer error: {:?}", e);
                    }
                }
            });
        }

        loop {
            // tokio::select! {
            //     e = self.cmd_rx => {
            //     }
            // }

            match self.cmd_rx.recv().await {
                None => return Ok(()),
                Some(StreamCmd::StartStream(id)) => {
                    //
                }
                Some(StreamCmd::StopStream(id)) => {
                    //
                }
            }
        }

        // unimplemented!()
        // Ok(())
    }

    // pub async fn add_stream(
    //     &mut self,
    //     id: PrinterId,
    //     handle: egui::TextureHandle,
    //     kill_rx: tokio::sync::oneshot::Receiver<()>,
    // ) -> Result<()> {
    //     // let stream = JpegStreamViewer::new(configs, id, handle, kill_rx).await?;
    //     // self.streams.insert(id, stream);
    //     unimplemented!()
    // }
}

/// https://github.com/greghesp/ha-bambulab/blob/main/custom_components/bambu_lab/pybambu/bambu_client.py#L68
pub struct JpegStreamViewer {
    config: Arc<RwLock<PrinterConfig>>,
    // addr: String,
    auth_data: Vec<u8>,
    tls_stream: tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
    buf: [u8; Self::READ_CHUNK_SIZE],
    // img_tx: tokio::sync::watch::Sender<Vec<u8>>,
    // ctx: egui::Context,
    handle: egui::TextureHandle,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
}

impl JpegStreamViewer {
    const JPEG_START: [u8; 4] = [0xff, 0xd8, 0xff, 0xe0];
    const JPEG_END: [u8; 2] = [0xff, 0xd9];
    const READ_CHUNK_SIZE: usize = 4096;

    pub async fn new(
        configs: ConfigArc,
        // config: Arc<RwLock<PrinterConfig>>,
        id: PrinterId,
        // img_tx: tokio::sync::watch::Sender<Vec<u8>>,
        // ctx: egui::Context,
        handle: egui::TextureHandle,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        let config = &configs.get_printer(&id).unwrap();
        let serial = config.read().await.serial.clone();
        let host = config
            .read()
            .await
            .host
            .clone()
            // .context("stream: missing host")?;
            ;
        let addr = format!("{}:6000", host);
        let access_code = config.read().await.access_code.clone();

        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add_parsable_certificates(
            rustls_native_certs::load_native_certs().expect("could not load platform certs"),
        );

        let client_config = rustls::ClientConfig::builder()
            // .with_root_certificates(root_cert_store)
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(crate::mqtt::NoCertificateVerification {
                serial: (*serial).clone(),
            }))
            .with_no_client_auth();

        let connector = rumqttc::tokio_rustls::TlsConnector::from(Arc::new(client_config));

        // debug!("Jpeg Viewer Connecting");
        let stream = tokio::net::TcpStream::connect(addr).await?;
        // debug!("Jpeg Viewer Connected");

        let domain = rustls::pki_types::ServerName::try_from(host).unwrap();
        let mut tls_stream = connector.connect(domain, stream).await?;
        // debug!("TLS handshake completed");

        let auth_data = {
            use byteorder::{LittleEndian, WriteBytesExt};

            let username = "bblp";

            let mut auth_data = vec![];
            auth_data.write_u32::<LittleEndian>(0x40).unwrap();
            auth_data.write_u32::<LittleEndian>(0x3000).unwrap();
            auth_data.write_u32::<LittleEndian>(0).unwrap();
            auth_data.write_u32::<LittleEndian>(0).unwrap();

            for &b in username.as_bytes() {
                auth_data.push(b);
            }
            for _ in 0..(32 - username.len()) {
                auth_data.push(0);
            }

            for &b in access_code.as_bytes() {
                auth_data.push(b);
            }
            for _ in 0..(32 - access_code.len()) {
                auth_data.push(0);
            }
            auth_data
        };

        Ok(Self {
            config: config.clone(),
            // config,
            // addr,
            auth_data,
            tls_stream,
            buf: [0u8; Self::READ_CHUNK_SIZE],
            // img_tx,
            handle,
            // ctx,
            kill_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        tokio::io::AsyncWriteExt::write_all(&mut self.tls_stream, &self.auth_data).await?;

        debug!("getting socket status");
        let status = self.tls_stream.get_ref().0.take_error();
        if !matches!(status, Ok(None)) {
            error!("socket status = {:?}", status);
            bail!("socket status = {:?}", status);
        }
        debug!("socket status ok, running loop");

        let mut payload_size = 0;
        let mut img_buf: Vec<u8> = vec![];
        let mut got_header = false;

        loop {
            self.buf.fill(0);
            let n = self.tls_stream.read(&mut self.buf).await?;

            if got_header {
                // debug!("extending image by {}", n);
                img_buf.extend_from_slice(&self.buf[..n]);

                if img_buf.len() > payload_size {
                    warn!(
                        "unexpected image payload received: {} > {}",
                        img_buf.len(),
                        payload_size,
                    );
                    got_header = false;
                    img_buf.clear();
                    // break;
                } else if img_buf.len() == payload_size {
                    if &img_buf[0..4] != &Self::JPEG_START {
                        warn!("missing jpeg start bytes");
                        break;
                    } else if &img_buf[payload_size - 2..payload_size - 0] != &Self::JPEG_END {
                        warn!("missing jpeg end bytes");
                        break;
                    }

                    // debug!("got image");
                    /// use image crate to write jpeg to file
                    // let mut f = std::fs::File::create("test.jpg")?;
                    // std::io::Write::write_all(&mut f, &img)?;
                    let image = match image::load_from_memory(&img_buf) {
                        Ok(image) => image,
                        Err(e) => {
                            error!("failed to load image: {}", e);
                            break;
                        }
                    };
                    let img_size = [image.width() as _, image.height() as _];
                    let image_buffer = image.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let img = egui::ColorImage::from_rgba_unmultiplied(img_size, pixels.as_slice());

                    self.handle.set(img, Default::default());

                    got_header = false;
                    img_buf.clear();
                }
            } else if n == 16 {
                // debug!("got header");
                // img.extend_from_slice(&buf);

                // payload_size = int.from_bytes(dr[0:3], byteorder='little')
                // payload_size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                payload_size =
                    <byteorder::LittleEndian as byteorder::ByteOrder>::read_u32(&self.buf[0..4])
                        as usize;

                // debug!("payload_size = {}", payload_size);
                got_header = true;
            }

            if n == 0 {
                debug!("wrong access code");
                break;
            }
        }

        Ok(())
    }
}
