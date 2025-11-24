//! This module deals with our shell-specific interaction, code generation,
//! and detection.
//!
//! See the module `init` for the implementation of `csm init` - the command
//! that sets up the user's shell for integration with `csm`.
//!
//! See the module `env` for the implementation of `csm env (de)activate`,
//! the whole reason we go through this effort.

use clap::ValueEnum;
use clap_complete::aot::Shell;
use log::{debug, warn};
use std::fmt;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessesToUpdate, System};

const BASH_WRAPPER: &str = include_str!("../shell/csm.bash");
const FISH_WRAPPER: &str = include_str!("../shell/csm.fish");
const PWSH_WRAPPER: &str = include_str!("../shell/csm.ps1");
const ZSH_WRAPPER: &str = include_str!("../shell/csm.zsh");

#[derive(Clone, Debug, ValueEnum)]
pub enum SupportedShell {
    #[value(aliases(["bash.exe"]))]
    Bash,
    #[value(aliases(["fish.exe"]))]
    Fish,
    #[value(aliases(["pwsh", "pwsh.exe", "powershell", "powershell.exe"]))]
    Powershell,
    #[value(aliases(["zsh.exe"]))]
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
    pub fn to_clap_complete_shell(&self) -> Shell {
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

    pub fn detect() -> Option<Self> {
        Self::from_parent_process().or_else(Self::from_env)
    }

    /// We expect the hook scripts to set $_CSM_SHELL, and we use this to know
    /// which shell to generate code for in commands like `csm env activate`.
    pub fn from_csm_hook() -> Option<Self> {
        let env_csm_shell = std::env::var("_CSM_SHELL").ok()?;
        Self::from_str(&env_csm_shell, false).ok()
    }

    /// The $PATH syntax is also shell-dependent; this function provides a way
    /// prepend a directory to it for the given shell.
    pub fn prepend_path(&self, path: &Path) -> String {
        let str_path = path.to_string_lossy();
        match self {
            Self::Bash | Self::Fish | Self::Zsh => {
                self.set_env_var("PATH", format!("{}:$PATH", str_path).as_ref())
            }
            Self::Powershell => self.set_env_var(
                "PATH",
                format!("{}$([IO.Path]::PathSeparator)$env:PATH", str_path).as_ref(),
            ),
        }
    }

    fn env_var_codegen(&self, key: &str, value: &str) -> String {
        match self {
            Self::Bash | Self::Zsh => format!("export {}=\"{}\";", key, value),
            Self::Fish => format!("set -g {} \"{}\";", key, value),
            Self::Powershell => format!("$env:{} = \"{}\";", key, value),
        }
    }

    pub fn set_env_var(&self, key: &str, new_value: &str) -> String {
        let mut out = String::new();

        // Back up the existing value iff we haven't done so already.
        let backup_key = format!("_CSM_{}_ORIG", key);
        match std::env::var(&backup_key) {
            Ok(_) | Err(std::env::VarError::NotUnicode(_)) => debug!(
                "Refusing to override saved original env var ${} for ${}",
                backup_key, key
            ),
            Err(std::env::VarError::NotPresent) => match std::env::var(key) {
                Ok(val) => {
                    debug!("Saving existing ${} to ${}", key, backup_key);
                    out.push_str(&self.env_var_codegen(&backup_key, &val));
                }
                Err(std::env::VarError::NotPresent) => {
                    debug!("No existing env var ${} found, not saving old value", key)
                }
                Err(std::env::VarError::NotUnicode(_)) => {
                    warn!("Existing ${} was not valid utf-8", key)
                }
            },
        }
        out.push_str(&self.env_var_codegen(key, new_value));
        out
    }

    pub fn unset_env_var(&self, key: &str) -> String {
        match self {
            Self::Bash | Self::Zsh => format!("unset {};", key),
            Self::Fish => format!("set -e {};", key),
            // Powershell will complain if the env var doesn't exist - but we don't care.
            Self::Powershell => format!("Remove-Item Env:{} -ErrorAction SilentlyContinue;", key),
        }
    }

    /// Emit the shell code to restore an environment variable that was backed
    /// up to $_CSM_<name>_ORIG by `set_env_var()`. If there was no matching
    /// backup variable found, return None.
    ///
    /// Invalid utf-8 backup variables are treated as not present.
    pub fn restore_env_var(&self, key: &str) -> Option<String> {
        let backup_key = format!("_CSM_{}_ORIG", key);
        match std::env::var(&backup_key) {
            Ok(orig_value) => {
                debug!("Restoring original value of ${} from ${}", key, backup_key);
                Some(self.set_env_var(key, &orig_value))
            }
            Err(std::env::VarError::NotPresent) => {
                debug!("No original value for ${} found in ${}", key, backup_key);
                None
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                warn!(
                    "Original value for ${} in ${} was not valid utf-8",
                    key, backup_key
                );
                None
            }
        }
    }

    /// Emit the shell code to restore an environment variable that was backed
    /// up, and then unset the backup variable.
    pub fn restore_and_unset_env_var(&self, key: &str) -> String {
        let mut out = String::new();
        // Try to restore the env var. If no original value was stored, then the
        // env var never existed in the first place, so just unset it.
        match self.restore_env_var(key) {
            Some(code) => out.push_str(&code),
            None => out.push_str(&self.unset_env_var(key)),
        }
        let backup_key = format!("_CSM_{}_ORIG", key);
        out.push_str(&self.unset_env_var(&backup_key));
        out
    }
}

pub struct ShellConfiguration {
    pub profile_file: &'static str,
    pub init_command: &'static str,
    pub wrapper: &'static str,
}

impl ShellConfiguration {
    pub fn from_supported_shell(shell: &SupportedShell) -> Self {
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
                wrapper: ZSH_WRAPPER,
            },
        }
    }

    fn persist_command(&self) -> String {
        format!("echo '{}' >> {}", self.init_command, self.profile_file)
    }

    pub fn instructions(&self) -> String {
        format!(
            r#"To set up csm in your current shell session, run the following:
    {}

If you add it to your shell profile ({}), the hook should automatically be
enabled for future shell sessions.

You could run the following command to add it automatically:

    {}"#,
            self.init_command,
            self.profile_file,
            self.persist_command()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supportedshell_prepend_path() {
        let test_path = PathBuf::from("/tmp/testing");
        assert!(
            SupportedShell::Bash
                .prepend_path(&test_path)
                .ends_with(";export PATH=\"/tmp/testing:$PATH\";")
        );
        assert!(
            SupportedShell::Fish
                .prepend_path(&test_path)
                .ends_with(";set -g PATH \"/tmp/testing:$PATH\";")
        );
        assert!(
            SupportedShell::Zsh
                .prepend_path(&test_path)
                .ends_with(";export PATH=\"/tmp/testing:$PATH\";")
        );
        assert!(
            SupportedShell::Powershell
                .prepend_path(&test_path)
                .ends_with(";$env:PATH = \"/tmp/testing$([IO.Path]::PathSeparator)$env:PATH\";")
        );
    }

    #[test]
    fn test_supportedshell_restore_env_var() {
        temp_env::with_var("_CSM_TEST_VAR_ORIG", Some("a value"), || {
            assert_eq!(
                SupportedShell::Bash.restore_env_var("TEST_VAR").unwrap(),
                "export TEST_VAR=\"a value\";"
            );
            assert_eq!(
                SupportedShell::Fish.restore_env_var("TEST_VAR").unwrap(),
                "set -g TEST_VAR \"a value\";"
            );
            assert_eq!(
                SupportedShell::Zsh.restore_env_var("TEST_VAR").unwrap(),
                "export TEST_VAR=\"a value\";"
            );
            assert_eq!(
                SupportedShell::Powershell
                    .restore_env_var("TEST_VAR")
                    .unwrap(),
                "$env:TEST_VAR = \"a value\";"
            );
        });
    }

    #[test]
    fn test_supportedshell_restore_and_unset_env_var() {
        assert_eq!(
            SupportedShell::Bash.restore_and_unset_env_var("TEST_VAR_RESTORE_AND_UNSET"),
            "unset TEST_VAR_RESTORE_AND_UNSET;unset _CSM_TEST_VAR_RESTORE_AND_UNSET_ORIG;"
        );
        assert_eq!(
            SupportedShell::Fish.restore_and_unset_env_var("TEST_VAR_RESTORE_AND_UNSET"),
            "set -e TEST_VAR_RESTORE_AND_UNSET;set -e _CSM_TEST_VAR_RESTORE_AND_UNSET_ORIG;"
        );
        assert_eq!(
            SupportedShell::Zsh.restore_and_unset_env_var("TEST_VAR_RESTORE_AND_UNSET"),
            "unset TEST_VAR_RESTORE_AND_UNSET;unset _CSM_TEST_VAR_RESTORE_AND_UNSET_ORIG;"
        );
        assert_eq!(
            SupportedShell::Powershell.restore_and_unset_env_var("TEST_VAR_RESTORE_AND_UNSET"),
            "Remove-Item Env:TEST_VAR_RESTORE_AND_UNSET -ErrorAction SilentlyContinue;Remove-Item \
             Env:_CSM_TEST_VAR_RESTORE_AND_UNSET_ORIG -ErrorAction SilentlyContinue;"
        );
    }
}
