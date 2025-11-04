#![allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::cmd::Command;
use std::path::PathBuf;
use tempfile::{Builder, TempDir};

#[derive(Debug)]
pub enum Error {
    Which(which::Error),
    IO(std::io::Error),
    Regex(regex::Error),
    GenericError(String),
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

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::GenericError(err)
    }
}

pub struct Csm {
    pub home_dir: TempDir,
}

impl Csm {
    pub fn new() -> Result<Csm, Error> {
        Ok(Self {
            home_dir: Builder::new().prefix("csm-").tempdir()?,
        })
    }

    pub fn command(&self) -> Command {
        let mut command = cargo_bin_cmd!();
        // Avoid reading real .csmrc
        command.env("HOME", self.home_dir.path());
        command.env("USERPROFILE", self.home_dir.path());

        command
    }

    pub fn write_csmrc(&self, config: &str) -> Result<(), std::io::Error> {
        std::fs::write(self.home_dir.path().join(".csmrc"), config)
    }
}

pub fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}
