use chrono::Local;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt,
    fmt::time::FormatTime,
    EnvFilter,
};
use tracing_subscriber::fmt::format::Writer;


/// Custom timer for IST (24-hour format)
struct IstTimer;

impl FormatTime for IstTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = Local::now();
        write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S"))
    }
}

/// Initializing structured, async tracing
pub fn init_tracing() -> WorkerGuard {
    // Async, non-blocking stdout writer, such that will not block the current thread.
    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

    // Log level configuration via RUST_LOG
    // Example:
    // RUST_LOG=ipl_auction=debug,tokio=info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ipl_auction=info"));

    fmt()
        .with_writer(non_blocking)      // async logging
        .with_env_filter(env_filter)    // runtime log level control
        .json()                         // structured logs in json format, we understand easily
        .with_timer(IstTimer)           // IST timestamp
        .with_file(true)                // source file
        .with_line_number(true)         // source line
        .with_current_span(true)        // attach span info
        .with_span_list(true)           // show parent spans
        .init();

    guard
}
