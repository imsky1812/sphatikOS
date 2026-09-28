//! `sp build`: build the workspace for the laptop or the phone.

use std::process::{Command, ExitCode};

use crate::cli::{BuildArgs, Target};

/// Rust target triple for the Redmi Note 9 Pro (musl matches the Alpine-based
/// system image; see the handbook toolchain table).
pub const CURTANA_TRIPLE: &str = "aarch64-unknown-linux-musl";

/// Arguments passed to `cargo` for a build request.
pub fn cargo_args(args: &BuildArgs) -> Vec<String> {
    let mut out: Vec<String> = match args.target {
        Target::Host => vec!["build".into()],
        Target::Curtana => vec!["zigbuild".into(), "--target".into(), CURTANA_TRIPLE.into()],
    };
    match &args.package {
        Some(pkg) => out.extend(["-p".into(), pkg.clone()]),
        None => out.push("--workspace".into()),
    }
    if args.release {
        out.push("--release".into());
    }
    out
}

/// Runs the build, echoing the exact command first.
pub fn run(args: &BuildArgs) -> ExitCode {
    let cargo_args = cargo_args(args);
    println!("$ cargo {}", cargo_args.join(" "));
    match Command::new("cargo").args(&cargo_args).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            if args.target == Target::Curtana {
                eprintln!(
                    "sp build: the curtana build needs cargo-zigbuild and Zig \
                     (`cargo install cargo-zigbuild`, `rustup target add {CURTANA_TRIPLE}`, \
                     and Zig from ziglang.org or `pip install ziglang`)."
                );
            }
            ExitCode::from(exit_byte(status.code()))
        }
        Err(err) => {
            eprintln!("sp build: could not start cargo: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Maps a child exit code to a non-zero byte for our own exit status.
fn exit_byte(code: Option<i32>) -> u8 {
    match code {
        Some(c) if (1..=255).contains(&c) => c as u8,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(target: Target, release: bool, package: Option<&str>) -> BuildArgs {
        BuildArgs {
            target,
            release,
            package: package.map(str::to_owned),
        }
    }

    #[test]
    fn host_debug_builds_the_workspace() {
        assert_eq!(
            cargo_args(&args(Target::Host, false, None)),
            ["build", "--workspace"]
        );
    }

    #[test]
    fn host_release_single_package() {
        assert_eq!(
            cargo_args(&args(Target::Host, true, Some("sphatik-comp"))),
            ["build", "-p", "sphatik-comp", "--release"]
        );
    }

    #[test]
    fn curtana_uses_zigbuild_and_the_musl_triple() {
        assert_eq!(
            cargo_args(&args(Target::Curtana, true, None)),
            [
                "zigbuild",
                "--target",
                "aarch64-unknown-linux-musl",
                "--workspace",
                "--release"
            ]
        );
    }

    #[test]
    fn exit_codes_are_never_zero_on_failure() {
        assert_eq!(exit_byte(Some(101)), 101);
        assert_eq!(exit_byte(Some(0)), 1);
        assert_eq!(exit_byte(Some(-1)), 1);
        assert_eq!(exit_byte(Some(300)), 1);
        assert_eq!(exit_byte(None), 1);
    }
}
