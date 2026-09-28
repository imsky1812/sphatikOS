//! Command-line definition for `sp`.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// The Sphatik OS developer tool: one command for every build, flash and
/// debug task.
#[derive(Debug, Parser)]
#[command(name = "sp", version, about, long_about = None)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// `sp` subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build the workspace for the laptop or the phone.
    Build(BuildArgs),
    /// Stream logs from the phone (needs SSH over USB, WP 1.7).
    Logs(LogsArgs),
    /// Print the fastboot commands to flash an image; never runs them.
    Flash(FlashArgs),
}

/// Where a build runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// This machine, with plain `cargo build`.
    Host,
    /// Redmi Note 9 Pro: aarch64-unknown-linux-musl via cargo-zigbuild.
    Curtana,
}

/// Arguments for `sp build`.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Build target.
    #[arg(long, value_enum, default_value_t = Target::Host)]
    pub target: Target,
    /// Build with the release profile.
    #[arg(long)]
    pub release: bool,
    /// Build only this crate (default: the whole workspace).
    #[arg(short, long, value_name = "CRATE")]
    pub package: Option<String>,
}

/// Arguments for `sp logs`.
#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Keep streaming new lines.
    #[arg(short, long)]
    pub follow: bool,
    /// Only show logs from this service, for example `sphatik-comp`.
    pub service: Option<String>,
}

/// Arguments for `sp flash`.
#[derive(Debug, Args)]
pub struct FlashArgs {
    /// Boot image to flash to the `boot` partition.
    #[arg(
        long,
        value_name = "IMG",
        default_value = "out/curtana/sphatik-curtana-boot.img"
    )]
    pub boot: PathBuf,
    /// Root filesystem image to flash to the `userdata` partition.
    #[arg(
        long,
        value_name = "IMG",
        default_value = "out/curtana/sphatik-curtana-userdata.img"
    )]
    pub userdata: PathBuf,
    /// Extra partition to flash, as NAME=IMG. Only boot, dtbo, vbmeta and
    /// userdata are accepted; calibration and radio partitions are refused.
    #[arg(long = "partition", value_name = "NAME=IMG")]
    pub extra: Vec<String>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn help_lists_every_command() {
        let help = Cli::command().render_help().to_string();
        for cmd in ["build", "logs", "flash"] {
            assert!(
                help.contains(cmd),
                "`sp --help` is missing `{cmd}`:\n{help}"
            );
        }
    }

    #[test]
    fn build_defaults_to_host_debug_workspace() {
        let cli = Cli::try_parse_from(["sp", "build"]).expect("parses");
        let Command::Build(args) = cli.command else {
            panic!("expected build");
        };
        assert_eq!(args.target, Target::Host);
        assert!(!args.release);
        assert_eq!(args.package, None);
    }

    #[test]
    fn build_accepts_curtana_release_and_package() {
        let cli = Cli::try_parse_from([
            "sp",
            "build",
            "--target",
            "curtana",
            "--release",
            "-p",
            "sphatik-comp",
        ])
        .expect("parses");
        let Command::Build(args) = cli.command else {
            panic!("expected build");
        };
        assert_eq!(args.target, Target::Curtana);
        assert!(args.release);
        assert_eq!(args.package.as_deref(), Some("sphatik-comp"));
    }

    #[test]
    fn logs_parses_follow_and_service() {
        let cli = Cli::try_parse_from(["sp", "logs", "-f", "sphatik-comp"]).expect("parses");
        let Command::Logs(args) = cli.command else {
            panic!("expected logs");
        };
        assert!(args.follow);
        assert_eq!(args.service.as_deref(), Some("sphatik-comp"));
    }

    #[test]
    fn unknown_target_is_rejected() {
        assert!(Cli::try_parse_from(["sp", "build", "--target", "pixel"]).is_err());
    }
}
