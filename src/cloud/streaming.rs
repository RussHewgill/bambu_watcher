use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use rumqttc::tokio_rustls::{self, rustls};
use std::{collections::HashMap, sync::Arc};
use tokio::{io::AsyncReadExt, sync::RwLock};

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::PrinterId,
};

#[derive(Debug, Clone)]
pub enum StreamCmd {
    ToggleStream(PrinterId),
    StartStream(PrinterId),
    StopStream(PrinterId),
    RestartStream(PrinterId),
}

#[derive(Debug, Clone)]
enum StreamMsg {
    Panic(PrinterId),
}

#[derive(Clone)]
pub struct WebcamTexture {
    pub enabled: bool,
    pub handle: egui::TextureHandle,
}

impl WebcamTexture {
    pub fn new(enabled: bool, handle: egui::TextureHandle) -> Self {
        Self { enabled, handle }
    }
}

pub struct StreamManager {
    configs: ConfigArc,
    // streams: HashMap<PrinterId, JpegStreamViewer>,
    handles: Arc<DashMap<PrinterId, WebcamTexture>>,
    kill_tx: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
    stream_tx: tokio::sync::mpsc::UnboundedSender<StreamMsg>,
    stream_rx: tokio::sync::mpsc::UnboundedReceiver<StreamMsg>,
    ctx: egui::Context,
}

impl StreamManager {
    pub fn new(
        configs: ConfigArc,
        // configs: ConfigArc,
        handles: Arc<DashMap<PrinterId, WebcamTexture>>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<StreamCmd>,
        ctx: egui::Context,
    ) -> Self {
        let (stream_tx, stream_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            configs,
            // streams: HashMap::new(),
            handles,
            kill_tx: HashMap::new(),
            cmd_rx,
            stream_rx,
            stream_tx,
            ctx,
        }
    }

    async fn spawn_worker(&mut self, id: PrinterId) -> Result<()> {
        let config = self
            .configs
            .get_printer(&id)
            .context("missing printer config")?;

        let host = config.read().await.host.clone();
        let enabled = !host.is_empty() && url::Url::parse(&format!("http://{}", &host)).is_ok();

        if !enabled {
            debug!("streaming disabled for printer: {:?}", id);
        } else {
            debug!("streaming enabled for printer: {:?}", id);
        }

        if !self.handles.contains_key(&id) {
            let image = egui::ColorImage::new([80, 80], egui::Color32::from_gray(220));
            let handle =
                self.ctx
                    .load_texture(format!("{}_texture", &id), image, Default::default());

            self.handles
                .insert(id.clone(), WebcamTexture::new(enabled, handle));
        }

        if !enabled {
            return Ok(());
        }

        let handle = self.handles.get(&id).unwrap().handle.clone();

        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        self.kill_tx.insert(id.clone(), kill_tx);

        let msg_tx = self.stream_tx.clone();

        tokio::task::spawn(async move {
            if let Ok(mut streamer) =
                JpegStreamViewer::new(id.clone(), config, handle, kill_rx, msg_tx.clone()).await
            {
                if let Err(e) = streamer.run().await {
                    error!("streamer error: {:?}", e);
                    msg_tx.send(StreamMsg::Panic(id)).unwrap();
                }
            }
        });

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        for id in self.configs.printer_ids_async().await {
            if let Err(e) = self.spawn_worker(id).await {
                error!("failed to spawn worker: {:?}", e);
            }
        }

        #[cfg(feature = "nope")]
        /// spawn worker tasks
        for id in self.configs.printer_ids_async().await {
            // debug!("streaming printer: {:?}", id);
            // let (enabled, handle) = self.handles.get_mut(&id).unwrap();

            // if !x.enabled {
            //     continue;
            // }
            // let configs2 = self.configs.clone();

            // let mut x = self.handles.get_mut(&id).unwrap();

            // let handle = x.handle.clone();

            let config = self.configs.get_printer(&id).unwrap();
            if config.read().await.host.is_empty() {
                debug!("streaming disabled for printer: {:?}", id);
                x.enabled = false;
                continue;
            } else {
                debug!("streaming enabled for printer: {:?}", id);
                x.enabled = true;
            }

            let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
            self.kill_tx.insert(id.clone(), kill_tx);

            tokio::task::spawn(async move {
                if let Ok(mut streamer) = JpegStreamViewer::new(config, id, handle, kill_rx).await {
                    if let Err(e) = streamer.run().await {
                        error!("streamer error: {:?}", e);
                    }
                }
            });
        }

        loop {
            tokio::select! {
                msg = self.stream_rx.recv() => {
                    match msg {
                        None => return Ok(()),
                        Some(StreamMsg::Panic(id)) => {
                            error!("streaming panic");
                            self.stop_stream(id.clone(), false).await;
                            self.start_stream(id, false).await;
                        }
                    }
                }

                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        None => return Ok(()),
                        Some(StreamCmd::StartStream(id)) => self.start_stream(id, true).await,
                        Some(StreamCmd::StopStream(id)) => self.stop_stream(id, true).await,
                        Some(StreamCmd::RestartStream(id)) => {
                            self.stop_stream(id.clone(), false).await;
                            self.start_stream(id, false).await;
                        }
                        Some(StreamCmd::ToggleStream(id)) => {
                            if self.kill_tx.contains_key(&id) {
                                self.stop_stream(id, true).await
                            } else {
                                self.start_stream(id, true).await
                            }
                        }
                    }
                }
            }
        }

        // unimplemented!()
        // Ok(())
    }

    async fn start_stream(&mut self, id: PrinterId, set_enabled: bool) {
        // debug!("starting stream: {:?}", id);
        if let Err(e) = self.spawn_worker(id.clone()).await {
            error!("failed to spawn worker: {:?}", e);
        }
        if set_enabled {
            let mut entry = self.handles.get_mut(&id).unwrap();
            entry.enabled = true;
        }
    }

    async fn stop_stream(&mut self, id: PrinterId, set_enabled: bool) {
        // debug!("stopping stream: {:?}", id);
        if let Some(kill_tx) = self.kill_tx.remove(&id) {
            let _ = kill_tx.send(());
        }
        if set_enabled {
            let mut entry = self.handles.get_mut(&id).unwrap();
            entry.enabled = false;
        }
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
    id: PrinterId,
    config: Arc<RwLock<PrinterConfig>>,
    // addr: String,
    auth_data: Vec<u8>,
    tls_stream: tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
    buf: [u8; Self::READ_CHUNK_SIZE],
    // img_tx: tokio::sync::watch::Sender<Vec<u8>>,
    // ctx: egui::Context,
    handle: egui::TextureHandle,
    kill_rx: tokio::sync::oneshot::Receiver<()>,
    msg_tx: tokio::sync::mpsc::UnboundedSender<StreamMsg>,
}

impl JpegStreamViewer {
    const JPEG_START: [u8; 4] = [0xff, 0xd8, 0xff, 0xe0];
    const JPEG_END: [u8; 2] = [0xff, 0xd9];
    const READ_CHUNK_SIZE: usize = 4096;

    const STREAM_TIMEOUT: u64 = 10;
    // const STREAM_TIMEOUT: u64 = 3;

    // async fn reset(mut self) -> Result<Self> {
    //     let Self {
    //         id,
    //         config,
    //         handle,
    //         kill_rx,
    //         ..
    //     } = self;
    //     Self::new(id, config, handle, kill_rx).await
    // }

    async fn new(
        // configs: ConfigArc,
        id: PrinterId,
        config: Arc<RwLock<PrinterConfig>>,
        // img_tx: tokio::sync::watch::Sender<Vec<u8>>,
        // ctx: egui::Context,
        handle: egui::TextureHandle,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
        msg_tx: tokio::sync::mpsc::UnboundedSender<StreamMsg>,
    ) -> Result<Self> {
        // let config = &configs.get_printer(&id).unwrap();
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
            id,
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
            msg_tx,
        })
    }

    async fn run(&mut self) -> Result<()> {
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

            let n = match tokio::time::timeout(
                tokio::time::Duration::from_secs(Self::STREAM_TIMEOUT),
                self.tls_stream.read(&mut self.buf),
            )
            .await
            {
                Ok(n) => n?,
                Err(_) => {
                    warn!("timeout reading from stream");
                    self.msg_tx.send(StreamMsg::Panic(self.id.clone())).unwrap();
                    bail!("timeout reading from stream");
                }
            };
            // let n = self.tls_stream.read(&mut self.buf).await?;

            if got_header {
                // debug!("extending image by {}", n);
                img_buf.extend_from_slice(&self.buf[..n]);

                if img_buf.len() > payload_size {
                    warn!(
                        "unexpected image payload received: {} > {}",
                        img_buf.len(),
                        payload_size,
                    );
                    // got_header = false;
                    // img_buf.clear();

                    /// not sure what the extra data is?
                    img_buf.truncate(payload_size);

                    // break;
                }
                if img_buf.len() == payload_size {
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
