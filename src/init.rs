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

use clap::Command;
use clap_complete::aot::{Shell, generate};
use log::{debug, error};
use std::process::ExitCode;
use sysinfo::{ProcessesToUpdate, System};

struct ShellConfiguration {
    profile_file: &'static str,
    init_command: &'static str,
}

impl ShellConfiguration {
    fn from_shell(shell: Shell) -> Self {
        match shell {
            Shell::PowerShell => Self {
                profile_file: "$PROFILE",
                init_command: "csm init powershell --code | Out-String | Invoke-Expression",
            },
            Shell::Bash => Self {
                profile_file: "~/.bashrc",
                init_command: "eval \"$(csm init bash --code)\"",
            },
            Shell::Zsh => Self {
                profile_file: "~/.zshrc",
                init_command: "eval \"$(csm init zsh --code)\"",
            },
            Shell::Fish => Self {
                profile_file: "~/.config/fish/config.fish",
                init_command: "csm init fish --code | source",
            },
            Shell::Elvish => Self {
                profile_file: "~/.config/elvish/rc.elv",
                init_command: "eval (csm init elvish --code)",
            },
            _ => Self {
                profile_file: "your shell profile",
                init_command: "eval \"$(csm init <shell> --code)\"",
            },
        }
    }

    fn persist_command(&self) -> String {
        format!("echo '{}' >> {}", self.init_command, self.profile_file)
    }

    fn print_instructions(&self) {
        println!("To set up csm in your current shell session, run the following:");
        println!("  {}", self.init_command);
        println!();
        println!(
            "If you add it to your shell profile ({}), the hook should ",
            self.profile_file
        );
        println!("be enabled in future shell sessions.");
        println!();
        println!("You can run this command to add it automatically:");
        println!("  {}", self.persist_command());
    }
}

fn shell_from_str(shell: &str) -> Option<Shell> {
    match shell {
        "bash" => Some(Shell::Bash),
        "elvish" => Some(Shell::Elvish),
        "fish" => Some(Shell::Fish),
        "powershell" | "pwsh" => Some(Shell::PowerShell),
        "zsh" => Some(Shell::Zsh),
        _ => None,
    }
}

fn shell_from_parent_process() -> Option<Shell> {
    let pid = sysinfo::get_current_pid().ok()?;
    let mut system = System::new();

    system.refresh_processes(ProcessesToUpdate::All, true);

    let parent_pid = system.process(pid)?.parent()?;
    let parent = system.process(parent_pid)?;
    let parent_name = parent.name().to_str()?;
    match shell_from_str(parent_name) {
        Some(shell) => {
            debug!("Detected {} from parent process name", shell);
            Some(shell)
        }
        None => {
            debug!("Did not detect a supported shell from parent process name");
            None
        }
    }
}

fn shell_from_env() -> Option<Shell> {
    match Shell::from_env() {
        Some(shell) => {
            debug!("Detected {} from $SHELL value", shell);
            Some(shell)
        }
        None => {
            debug!("Did not detect a supported shell from $SHELL");
            None
        }
    }
}

fn detect_shell() -> Option<Shell> {
    shell_from_parent_process().or_else(shell_from_env)
}

pub fn run(
    _config: Config,
    specified_shell: Option<Shell>,
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
        None => match detect_shell() {
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

    if code {
        generate(
            shell,
            cmd,
            cmd.get_name().to_string(),
            &mut std::io::stdout(),
        );
    } else {
        ShellConfiguration::from_shell(shell).print_instructions();
    }

    ExitCode::SUCCESS
}
