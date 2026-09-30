//! Structured file + in-memory ring logging. Audio content is never logged, and
//! the audio callbacks never call into this module.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::OnceLock;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static RING: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
const RING_CAP: usize = 500;

struct RingWriter;
impl std::io::Write for RingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(buf).trim_end().to_string();
        if !line.is_empty() {
            let mut r = RING.get_or_init(Default::default).lock();
            if r.len() >= RING_CAP {
                r.pop_front();
            }
            r.push_back(line);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,auralis_lib=debug"));
    let appender = tracing_appender::rolling::daily(crate::persistence::log_dir(), "auralis.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);
    let res = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(nb).with_ansi(false))
        .with(
            fmt::layer()
                .with_writer(|| RingWriter)
                .with_ansi(false)
                .without_time(),
        )
        .try_init();
    res.ok().map(|_| guard)
}

/// Most recent log lines, for the Diagnostics page.
pub fn recent(n: usize) -> Vec<String> {
    let r = RING.get_or_init(Default::default).lock();
    r.iter().rev().take(n).rev().cloned().collect()
}
