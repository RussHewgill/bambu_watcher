use anyhow::{anyhow, bail, ensure, Context, Result};
use suppaftp::{native_tls::TlsConnector, NativeTlsConnector};
use tracing::{debug, error, info, trace, warn};

use crate::config::PrinterConfig;

pub fn get_gcode_thumbnail(printer_cfg: &PrinterConfig, gcode: &str) -> Result<Vec<u8>> {
    /// this is icky, but it's only usable on LAN at least
    debug!("building ctx");
    let ctx = NativeTlsConnector::from(
        TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap(),
    );

    let addr = format!("{}:{}", printer_cfg.host, 990);
    debug!("addr = {}", addr);

    /// explicit doesn't work for some reason
    debug!("connecting");
    let mut conn =
        suppaftp::NativeTlsFtpStream::connect_secure_implicit(&addr, ctx, &printer_cfg.host)
            .unwrap();

    debug!("connected to server");
    conn.login("bblp", &printer_cfg.access_code).unwrap();

    let files = conn.list(Some("image"))?;

    use std::str::FromStr;
    use suppaftp::list::File;

    for file in files {
        // debug!("file = {:?}", file);
        let f = File::from_str(&file).unwrap();
        let name = f.name();
        let modified = f.modified();
        debug!("name = {}, modified = {:?}", name, modified);
    }

    // debug!("getting buf");
    // let buf = ftp_stream.retr_as_buffer(gcode)?;
    // debug!("got buf");

    // let mut zip = zip::ZipArchive::new(buf)?;

    // debug!("got zip");

    let _ = conn.quit();

    // let mut ftp = FtpStream::connect(printer_cfg.host)?;
    // ftp.login("bblp", &printer_cfg.access_code)?;
    // let mut reader = ftp.get(gcode)?;
    // let mut buf = Vec::new();
    // reader.read_to_end(&mut buf)?;
    // Ok(buf)
    unimplemented!()
}
