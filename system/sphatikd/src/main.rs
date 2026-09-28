//! `sphatikd`: Sphatik OS system services.
//!
//! Settings, permissions, notifications, apps, live activities and power,
//! exposed on the system bus as `org.sphatik.Daemon`
//! (see `docs/api/sphatikd-dbus.md`). Arrives in WP 4.8.
//!
//! System services must never panic: return errors and log them.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

/// One-line identification printed at start-up.
fn banner() -> String {
    format!("sphatikd {}", env!("CARGO_PKG_VERSION"))
}

fn main() {
    println!("{}", banner());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_names_the_binary_and_version() {
        assert_eq!(banner(), format!("sphatikd {}", env!("CARGO_PKG_VERSION")));
    }
}
