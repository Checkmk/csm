#![allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379

use assert_cmd::cargo::{self, cargo_bin_cmd};
use assert_cmd::cmd::Command;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs, io};
use tempfile::{Builder, NamedTempFile, TempDir, TempPath};

#[cfg(windows)]
pub const EOL: &str = "\r\n";
#[cfg(not(windows))]
pub const EOL: &str = "\n";

#[derive(Debug)]
pub enum Error {
    Which(which::Error),
    IO(io::Error),
    Regex(regex::Error),
    Generic(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
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
        Self::Generic(err)
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

    pub fn ext_command(&self, bin: &str) -> Command {
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
        let path = match env::var("PATH") {
            Ok(path) => format!("{}{}{}", path, separator, csm_bin_dir),
            Err(_) => csm_bin_dir,
        };
        command.env("PATH", path);

        command
    }

    /// Similar to `ext_command`, but stores a script in a temporary file
    /// (outside of the temporary home directory).
    ///
    /// This primarily exists because when reading from stdin, PowerShell will
    /// by default insist on displaying its prompt and name banner, unlike every
    /// other shell in existence.
    pub fn run_script(&self, bin: &str, script: &str, suffix: &str) -> io::Result<ScriptCommand> {
        let mut tmpfile = NamedTempFile::with_suffix(suffix)?;
        writeln!(tmpfile, "{}", script)?;
        let tmpfile_path = tmpfile.into_temp_path();
        let mut cmd = self.ext_command(bin);
        cmd.arg(&tmpfile_path);
        Ok(ScriptCommand {
            command: cmd,
            _tmpfile: tmpfile_path,
        })
    }

    pub fn write_csmrc(&self, config: &str) -> Result<(), io::Error> {
        fs::write(self.home_dir.path().join(".csmrc"), config)
    }
}

/// This exists only to make sure that the tmpfile doesn't get dropped
/// prematurely from Csm.run_script(). The tmpfile will get dropped when the
/// reference to it goes away - so we need a way to keep the reference around
/// with the Command that we return.
pub struct ScriptCommand {
    pub command: Command,
    _tmpfile: TempPath,
}

impl std::ops::Deref for ScriptCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl std::ops::DerefMut for ScriptCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

pub fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}
