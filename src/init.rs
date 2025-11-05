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

use clap::{Command, ValueEnum};
use clap_complete::aot::{Shell, generate};
use log::{debug, error};
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;
use sysinfo::{ProcessesToUpdate, System};

const BASH_WRAPPER: &str = include_str!("../shell/csm.bash");
const FISH_WRAPPER: &str = include_str!("../shell/csm.fish");
const PWSH_WRAPPER: &str = include_str!("../shell/csm.ps1");

#[derive(Clone, Debug, ValueEnum)]
pub enum SupportedShell {
    Bash,
    Fish,
    #[value(alias("pwsh"))]
    Powershell,
    Zsh,
}

impl fmt::Display for SupportedShell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Fish => write!(f, "fish"),
            Self::Powershell => write!(f, "powershell"),
            Self::Zsh => write!(f, "zsh"),
        }
    }
}

impl SupportedShell {
    fn to_clap_complete_shell(&self) -> Shell {
        match self {
            Self::Bash => Shell::Bash,
            Self::Fish => Shell::Fish,
            Self::Powershell => Shell::PowerShell,
            Self::Zsh => Shell::Zsh,
        }
    }

    fn from_parent_process() -> Option<Self> {
        let pid = sysinfo::get_current_pid().ok()?;
        let mut system = System::new();

        system.refresh_processes(ProcessesToUpdate::All, true);

        let parent_pid = system.process(pid)?.parent()?;
        let parent = system.process(parent_pid)?;
        let parent_name = parent.name().to_str()?;
        match Self::from_str(parent_name, true) {
            Ok(shell) => {
                debug!("Detected {} from parent process name", shell);
                Some(shell)
            }
            Err(s) => {
                debug!("Could not convert parent process name to shell: {}", s);
                None
            }
        }
    }

    fn from_env() -> Option<Self> {
        let env_shell = std::env::var("SHELL").ok()?;
        let path = PathBuf::from(env_shell);
        match Self::from_str(
            &path.components().next_back()?.as_os_str().to_string_lossy(),
            true,
        ) {
            Ok(shell) => {
                debug!("Detected {} from $SHELL", shell);
                Some(shell)
            }
            Err(s) => {
                debug!("Could not convert $SHELL value to shell: {}", s);
                None
            }
        }
    }

    fn detect() -> Option<Self> {
        Self::from_parent_process().or_else(Self::from_env)
    }
}

struct ShellConfiguration {
    profile_file: &'static str,
    init_command: &'static str,
    wrapper: &'static str,
}

impl ShellConfiguration {
    fn from_supported_shell(shell: &SupportedShell) -> Self {
        match shell {
            SupportedShell::Bash => Self {
                profile_file: "~/.bashrc",
                init_command: "eval \"$(csm init bash --code)\"",
                wrapper: BASH_WRAPPER,
            },
            SupportedShell::Fish => Self {
                profile_file: "~/.config/fish/config.fish",
                init_command: "csm init fish --code | source",
                wrapper: FISH_WRAPPER,
            },
            SupportedShell::Powershell => Self {
                profile_file: "$PROFILE",
                init_command: "csm init powershell --code | Out-String | Invoke-Expression",
                wrapper: PWSH_WRAPPER,
            },
            SupportedShell::Zsh => Self {
                profile_file: "~/.zshrc",
                init_command: "eval \"$(csm init zsh --code)\"",
                wrapper: BASH_WRAPPER,
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

    let config = ShellConfiguration::from_supported_shell(&shell);

    if code {
        generate(
            shell.to_clap_complete_shell(),
            cmd,
            cmd.get_name().to_string(),
            &mut std::io::stdout(),
        );
        println!();
        println!("{}", config.wrapper);
    } else {
        config.print_instructions();
    }

    ExitCode::SUCCESS
}
