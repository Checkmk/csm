#![allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379

use assert_cmd::cargo::{self, cargo_bin_cmd};
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

    /// Try to isolate calls to csm and micromamba from the actual system as
    /// much as possible, even if the user running the test has some env vars
    /// already set.
    fn prepare_command(&self, command: &mut Command) {
        command.env("HOME", self.home_dir.path());
        command.env("USERPROFILE", self.home_dir.path());
        command.env_remove("CONDA_PREFIX");
        command.env_remove("MAMBA_ROOT_PREFIX");
        command.current_dir(self.home_dir.path());
    }

    pub fn command(&self) -> Command {
        let mut command = cargo_bin_cmd!();
        self.prepare_command(&mut command);
        command
    }

    pub fn ext_command(&self, bin: PathBuf) -> Command {
        let mut command = Command::new(bin);
        self.prepare_command(&mut command);

        // Add csm into $PATH, in case we want to use it from a sh -c or similar
        let csm_path = cargo::cargo_bin!();
        let csm_bin_dir = csm_path
            .parent()
            .expect("Cannot get csm binary directory")
            .to_string_lossy()
            .replace("\\", "/");
        let separator = if cfg!(windows) { ";" } else { ":" };
        let path = match std::env::var("PATH") {
            Ok(path) => format!("{}{}{}", path, separator, csm_bin_dir),
            Err(_) => csm_bin_dir,
        };
        command.env("PATH", path);

        command
    }

    pub fn write_csmrc(&self, config: &str) -> Result<(), std::io::Error> {
        std::fs::write(self.home_dir.path().join(".csmrc"), config)
    }
}

pub fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}
