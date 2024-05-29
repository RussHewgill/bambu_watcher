use tracing_log::LogTracer;
use tracing_subscriber::{prelude::*, registry::Registry, EnvFilter};

#[cfg(feature = "nope")]
pub fn init_logs() {
    tracing_subscriber::fmt()
        .with_env_filter("bambu_watcher=debug")
        // .with_env_filter("derp_learning=trace, derp_learning_test=trace")
        // .with_max_level(tracing::Level::DEBUG)
        .without_time()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_level(true)
        .compact()
        .finish()
        .try_init()
        .unwrap();
}

pub fn init_logs() {
    let _ = std::fs::rename("output.log", "output_prev.log");
    let _ = std::fs::remove_file("output.log");

    let trace_file =
        tracing_appender::rolling::never(".", "output.log").with_max_level(tracing::Level::TRACE);

    // LogTracer::init().unwrap();

    let file_layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(trace_file)
        .with_file(true)
        .with_ansi(false)
        .with_line_number(true)
        .with_target(true)
        .with_level(true)
        .compact()
        .with_filter(tracing_subscriber::filter::EnvFilter::new(
            "info,bambu_watcher=trace,eframe=warn,wgpu=warn",
        ));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .without_time()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .compact()
        .with_filter(tracing_subscriber::filter::EnvFilter::new(
            // "info,bambu_watcher=debug,eframe=warn,wgpu=warn",
            "warn,bambu_watcher=debug,eframe=warn,wgpu=warn",
        ));

    let subscriber = tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .try_init()
        .unwrap();
}
