use std::path::{Path, PathBuf};

#[derive(Debug)]
#[allow(dead_code)]
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

#[allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379
pub fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[allow(dead_code)] // https://github.com/rust-lang/rust/issues/46379
pub fn write_csmrc(home_dir: &Path, config: &str) -> Result<(), std::io::Error> {
    std::fs::write(home_dir.join(".csmrc"), config)
}
