use std::process::{ExitCode, ExitStatus, Output};

/// The result from trying to shell out to `micromamba`.
///
/// It would be better if we could "accumulate" errors as we try different
/// fallbacks to run `micromamba`, something like Result/Either, but with an
/// accumulating Applicative on the error side, akin to the "validation" package
/// in Haskell. Alas, this does not seem to exist in Rust, so we drop the errors
/// as we try to determine a working `micromamba` and just report whether or not
/// we were able to do so at the end. (Of course, we log along the way in
/// `micromamba()`.)
pub enum MicromambaResult {
    /// We were run in no-op mode, so we didn't actually call out to it
    Noop,
    /// We were able to successfully call it and get a result, though we streamed
    /// the output and did not save it.
    StreamedOutput(ExitStatus),
    /// We were able to successfully call it and get a result, capturing output.
    CapturedOutput(Output),
    /// We were unable to find or create a working `micromamba`
    NotFound,
    /// We found a micromamba binary, but could not run it
    CouldNotRun,
}

impl MicromambaResult {
    pub fn exit_code(&self) -> ExitCode {
        let to_code = |status: &ExitStatus| {
            status
                .code()
                .map(|c| ExitCode::from(c as u8))
                .unwrap_or(ExitCode::FAILURE)
        };

        match self {
            Self::StreamedOutput(exit_status) => to_code(exit_status),
            Self::CapturedOutput(output) => to_code(&output.status),
            Self::Noop => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        }
    }
}

impl From<MicromambaResult> for Result<(), ExitCode> {
    fn from(result: MicromambaResult) -> Self {
        match result.exit_code() {
            ExitCode::SUCCESS => Ok(()),
            e => Err(e),
        }
    }
}
