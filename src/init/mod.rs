//! We use [`clap_complete`](https://docs.rs/clap_complete/latest/) for
//! generating shell completions for `csm. This has the benefit of giving us a
//! solidified list of shells we support for the shell hook. We simply support
//! the ones that `clap_complete` supports.
//!
//! We do go through a bit more trouble than `clap_complete`'s `from_env()` to
//! try to determine the shell in use. Namely, we take into account the parent
//! process that spawned `csm` first. If that is of no help, then we fall bacck
//! to `from_env()` which will try to determine it from $SHELL.
//!
//! Ultimately, we expect the user to specify the shell explicitly when loading
//! the shell hook; the auto-detect is just to try to help the user out if they
//! run `csm init`.
use crate::csmrc::Config;
use crate::shell::{ShellConfiguration, SupportedShell};

use clap::Command;
use clap_complete::aot::generate;
use log::{debug, error};
use std::process::ExitCode;

fn shell_hook(
    shell_config: ShellConfiguration,
    shell: SupportedShell,
    cmd: &mut Command,
) -> String {
    let mut hook = Vec::new();
    generate(
        shell.to_clap_complete_shell(),
        cmd,
        cmd.get_name().to_string(),
        &mut hook,
    );
    let mut result = String::from_utf8(hook).unwrap_or_default();
    result.push('\n');
    result.push_str(shell_config.wrapper);
    result
}

pub fn run(
    _config: Config,
    specified_shell: Option<SupportedShell>,
    code: bool,
    cmd: &mut Command,
) -> ExitCode {
    let shell = match specified_shell {
        Some(shell) => {
            debug!("Using shell {}, explicitly requested by user", shell);
            shell
        }
        None if code => {
            error!(
                "Shell must be explicitly specified to generate code. Use 'csm init <shell> --code'"
            );
            return ExitCode::FAILURE;
        }
        None => match SupportedShell::detect() {
            Some(detected_shell) => {
                if !code {
                    println!("We think you are using {}.", detected_shell);
                    println!("If this is wrong, use 'csm init <shell>' to specify your shell.");
                    println!();
                }
                detected_shell
            }
            None => {
                error!(
                    "Could not auto-detect your shell. Please specify it explicitly. See 'csm help init'."
                );
                return ExitCode::FAILURE;
            }
        },
    };

    let shell_config = ShellConfiguration::from_supported_shell(&shell);

    if code {
        let hook_code = shell_hook(shell_config, shell, cmd);
        println!("{}", hook_code);
    } else {
        println!("{}", shell_config.instructions());
    }

    ExitCode::SUCCESS
}
