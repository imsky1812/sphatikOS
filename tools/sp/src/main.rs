//! `sp`: the Sphatik OS developer tool.
//!
//! One command for every build, flash and debug task. The command-line
//! skeleton (`build`, `logs`, `flash`) arrives in WP 1.3.

/// One-line identification printed at start-up.
fn banner() -> String {
    format!("sp {}", env!("CARGO_PKG_VERSION"))
}

fn main() {
    println!("{}", banner());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_names_the_binary_and_version() {
        assert_eq!(banner(), format!("sp {}", env!("CARGO_PKG_VERSION")));
    }
}
