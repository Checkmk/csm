pub mod csmrc;
pub mod env;
pub mod init;
pub mod micromamba;
pub mod robot;
pub mod shell;

use log::error;
use std::process::ExitCode;

pub trait CSMResult {
    fn finish(&self) -> ExitCode;
}

impl CSMResult for ExitCode {
    fn finish(&self) -> Self {
        *self
    }
}

impl<T> CSMResult for Result<T, ExitCode> {
    fn finish(&self) -> ExitCode {
        match self {
            Ok(_) => ExitCode::SUCCESS,
            Err(code) => *code,
        }
    }
}

impl<T> CSMResult for Result<T, std::io::Error> {
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
