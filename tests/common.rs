#![allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug)]
pub enum Error {
    Which(which::Error),
    IO(std::io::Error),
    Regex(regex::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<which::Error> for Error {
    fn from(err: which::Error) -> Self {
        Self::Which(err)
    }
}

impl From<regex::Error> for Error {
    fn from(err: regex::Error) -> Self {
        Self::Regex(err)
    }
}

pub struct Csm {
    pub command: Command,
    pub home_dir: TempDir,
}

pub fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

pub fn write_csmrc(home_dir: &Path, config: &str) -> Result<(), std::io::Error> {
    std::fs::write(home_dir.join(".csmrc"), config)
}

pub fn csm() -> Result<Csm, Error> {
    let temp_dir = TempDir::new()?;

    let mut command = cargo_bin_cmd!();
    command.env("HOME", temp_dir.path()); // Avoid reading real .csmrc
    command.env("USERPROFILE", temp_dir.path());

    let csm = Csm {
        command: command,
        home_dir: temp_dir,
    };

    Ok(csm)
}
