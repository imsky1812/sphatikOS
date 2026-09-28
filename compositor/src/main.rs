//! `sphatik-comp`: the Sphatik OS Wayland compositor.
//!
//! Owns the display and input, composites every window and draws the shell
//! and all glass in one GPU pass per frame (ADR 0002, ADR 0007). The winit
//! backend arrives in WP 2.1.

/// One-line identification printed at start-up.
fn banner() -> String {
    format!("sphatik-comp {}", env!("CARGO_PKG_VERSION"))
}

fn main() {
    println!("{}", banner());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_names_the_binary_and_version() {
        assert_eq!(
            banner(),
            format!("sphatik-comp {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
