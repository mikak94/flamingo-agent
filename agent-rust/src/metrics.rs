use chrono::{DateTime, SecondsFormat, Utc};

use crate::error::Result;
use crate::platform;

#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub utc: DateTime<Utc>,
    pub rss_bytes: u64,
}

impl Metrics {
    pub fn collect() -> Result<Self> {
        Ok(Metrics {
            utc: Utc::now(),
            rss_bytes: platform::process_rss_bytes()?,
        })
    }

    /// The single argument handed to the child. No spaces or quotes, so it
    /// survives any command line as one token.
    pub fn to_arg(self) -> String {
        format!("utc={};rss_bytes={}", format_utc(self.utc), self.rss_bytes)
    }

    pub fn to_display(self) -> String {
        format!(
            "utc={} rss={} ({} bytes)",
            format_utc(self.utc),
            human_bytes(self.rss_bytes),
            self.rss_bytes
        )
    }
}

/// RFC 3339 aka ISO string `2026-09-15T19:20:31.123Z`.
pub fn format_utc(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
