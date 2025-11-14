pub mod csmrc;
pub mod env;
pub mod init;
pub mod micromamba;
pub mod robot;
pub mod shell;

use log::error;
use std::fmt;
use std::process::ExitCode;

pub trait CSMResult {
    fn finish(&self) -> ExitCode;
}

impl CSMResult for ExitCode {
    fn finish(&self) -> Self {
        *self
    }
}

impl<T, E: fmt::Display> CSMResult for Result<T, E> {
    fn finish(&self) -> ExitCode {
        match self {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                error!("{}", e);
                ExitCode::FAILURE
            }
        }
    }
}
