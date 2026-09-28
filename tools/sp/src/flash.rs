//! `sp flash`: print the fastboot commands that flash a Sphatik image.
//!
//! This module never runs fastboot or any other command. It only prints the
//! exact commands for the owner to review and run. Calibration, radio and
//! bootloader partitions are refused outright: flashing or erasing them can
//! permanently lose the IMEI, Wi-Fi or sensor calibration, or brick the phone.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::FlashArgs;

/// Partitions `sp flash` may ever name.
pub const ALLOWED: &[&str] = &["boot", "dtbo", "vbmeta", "userdata"];

/// Partitions that must never be flashed or erased from Sphatik tooling.
pub const PROTECTED: &[&str] = &[
    // Calibration and radio data, unique to each phone.
    "persist",
    "persistbak",
    "modem",
    "modemst1",
    "modemst2",
    "fsg",
    "fsc",
    "efs",
    "dsp",
    "bluetooth",
    "sec",
    // Bootloader chain; a bad write here can hard-brick the phone.
    "xbl",
    "xbl_config",
    "abl",
    "tz",
    "hyp",
    "aop",
    "devcfg",
    "keymaster",
    "qupfw",
    "uefisecapp",
    "imagefv",
    "ddr",
];

/// Why a flash plan was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum FlashError {
    /// The partition holds calibration, radio or bootloader data.
    Protected(String),
    /// The partition is not in [`ALLOWED`].
    NotAllowed(String),
    /// A `--partition` value was not of the form `NAME=IMG`.
    BadSpec(String),
    /// The same partition was named twice.
    Duplicate(String),
}

impl fmt::Display for FlashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protected(p) => write!(
                f,
                "refusing `{p}`: it holds calibration, radio or bootloader data and must never be flashed or erased"
            ),
            Self::NotAllowed(p) => write!(
                f,
                "refusing `{p}`: sp flash only writes {}",
                ALLOWED.join(", ")
            ),
            Self::BadSpec(s) => write!(f, "expected NAME=IMG, got `{s}`"),
            Self::Duplicate(p) => write!(f, "partition `{p}` is named more than once"),
        }
    }
}

/// Lower-cases a partition name and strips an A/B slot suffix.
fn normalise(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    match lower
        .strip_suffix("_a")
        .or_else(|| lower.strip_suffix("_b"))
    {
        Some(base) => base.to_owned(),
        None => lower,
    }
}

/// Accepts a partition only if it is allowed and not protected.
pub fn check_partition(name: &str) -> Result<String, FlashError> {
    let base = normalise(name);
    if PROTECTED.contains(&base.as_str()) {
        return Err(FlashError::Protected(name.to_owned()));
    }
    if !ALLOWED.contains(&base.as_str()) {
        return Err(FlashError::NotAllowed(name.to_owned()));
    }
    Ok(base)
}

/// Parses a `NAME=IMG` pair from `--partition`.
fn parse_extra(spec: &str) -> Result<(String, PathBuf), FlashError> {
    match spec.split_once('=') {
        Some((name, img)) if !name.trim().is_empty() && !img.trim().is_empty() => {
            Ok((check_partition(name)?, PathBuf::from(img.trim())))
        }
        _ => Err(FlashError::BadSpec(spec.to_owned())),
    }
}

/// Quotes a path for a POSIX or PowerShell command line if it has spaces.
fn quoted(path: &Path) -> String {
    let s = path.display().to_string();
    if s.contains(' ') {
        format!("\"{s}\"")
    } else {
        s
    }
}

/// Builds the ordered list of commands for the owner to run.
pub fn plan(args: &FlashArgs) -> Result<Vec<String>, FlashError> {
    let mut images: Vec<(String, PathBuf)> = vec![("boot".into(), args.boot.clone())];
    for spec in &args.extra {
        images.push(parse_extra(spec)?);
    }
    images.push(("userdata".into(), args.userdata.clone()));

    for (i, (name, _)) in images.iter().enumerate() {
        if images[..i].iter().any(|(other, _)| other == name) {
            return Err(FlashError::Duplicate(name.clone()));
        }
    }

    let mut cmds = vec![
        "fastboot devices".to_owned(),
        "fastboot getvar product    # must report curtana".to_owned(),
    ];
    cmds.extend(
        images
            .iter()
            .map(|(name, img)| format!("fastboot flash {name} {}", quoted(img))),
    );
    cmds.push("fastboot reboot".to_owned());
    Ok(cmds)
}

/// Prints the flash plan. Never executes anything.
pub fn run(args: &FlashArgs) -> ExitCode {
    let cmds = match plan(args) {
        Ok(cmds) => cmds,
        Err(err) => {
            eprintln!("sp flash: {err}");
            return ExitCode::FAILURE;
        }
    };
    println!("sp flash never runs these. Boot the phone to fastboot (Volume Down + Power),");
    println!("check each line, then run them yourself:\n");
    for cmd in &cmds {
        println!("    {cmd}");
    }
    let missing: Vec<&PathBuf> = [&args.boot, &args.userdata]
        .into_iter()
        .filter(|img| !img.exists())
        .collect();
    if !missing.is_empty() {
        println!();
        for img in missing {
            println!("note: {} does not exist yet.", img.display());
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(extra: &[&str]) -> FlashArgs {
        FlashArgs {
            boot: PathBuf::from("out/boot.img"),
            userdata: PathBuf::from("out/userdata.img"),
            extra: extra.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn every_protected_partition_is_refused_in_any_case_or_slot() {
        for p in PROTECTED {
            for name in [
                p.to_string(),
                p.to_ascii_uppercase(),
                format!("{p}_a"),
                format!("{p}_b"),
            ] {
                assert_eq!(
                    check_partition(&name),
                    Err(FlashError::Protected(name.clone())),
                    "{name} must be refused"
                );
            }
        }
    }

    #[test]
    fn protected_and_allowed_lists_do_not_overlap() {
        for p in ALLOWED {
            assert!(!PROTECTED.contains(p), "{p} is both allowed and protected");
        }
    }

    #[test]
    fn unknown_partitions_are_refused() {
        for name in ["system", "vendor", "recovery", "cust", "misc", ""] {
            assert!(check_partition(name).is_err(), "{name:?} must be refused");
        }
    }

    #[test]
    fn allowed_partitions_pass() {
        for name in ALLOWED {
            assert_eq!(check_partition(name), Ok((*name).to_owned()));
        }
        assert_eq!(check_partition("Boot_a"), Ok("boot".to_owned()));
    }

    #[test]
    fn default_plan_flashes_boot_then_userdata_and_reboots() {
        assert_eq!(
            plan(&args(&[])).expect("valid plan"),
            [
                "fastboot devices",
                "fastboot getvar product    # must report curtana",
                "fastboot flash boot out/boot.img",
                "fastboot flash userdata out/userdata.img",
                "fastboot reboot",
            ]
        );
    }

    #[test]
    fn extras_go_between_boot_and_userdata() {
        let cmds =
            plan(&args(&["dtbo=out/dtbo.img", "vbmeta=my images/vbmeta.img"])).expect("valid plan");
        assert_eq!(cmds[3], "fastboot flash dtbo out/dtbo.img");
        assert_eq!(cmds[4], "fastboot flash vbmeta \"my images/vbmeta.img\"");
        assert_eq!(cmds[5], "fastboot flash userdata out/userdata.img");
    }

    #[test]
    fn protected_extra_refuses_the_whole_plan() {
        assert_eq!(
            plan(&args(&["modem=evil.img"])),
            Err(FlashError::Protected("modem".to_owned()))
        );
        assert_eq!(
            plan(&args(&["PERSIST=x.img"])),
            Err(FlashError::Protected("PERSIST".to_owned()))
        );
    }

    #[test]
    fn malformed_and_duplicate_extras_are_refused() {
        assert_eq!(
            plan(&args(&["dtbo"])),
            Err(FlashError::BadSpec("dtbo".to_owned()))
        );
        assert_eq!(
            plan(&args(&["=x.img"])),
            Err(FlashError::BadSpec("=x.img".to_owned()))
        );
        assert_eq!(
            plan(&args(&["boot=other.img"])),
            Err(FlashError::Duplicate("boot".to_owned()))
        );
    }
}
