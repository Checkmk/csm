mod common;

use common::Error;

use clap::ValueEnum;
use predicates::prelude::*;

/// Test `csm init <shell>`
#[test]
fn test_csm_init_shell() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(["init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">> ~/.bashrc"));

    csm.command()
        .args(["init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">> ~/.zshrc"));

    csm.command()
        .args(["init", "unsupported"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values:"));
    Ok(())
}

/// Test `csm init <shell> --code` output
#[test]
fn test_csm_init_shell_code() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(["init", "bash", "--code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPREPLY"))
        .stdout(predicate::str::contains("complete -F"));

    csm.command()
        .args(["init", "zsh", "--code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));

    csm.command()
        .args(["init", "unsupported", "--code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values:"));
    Ok(())
}

/// Test suggested `csm init <shell> --code` evaluation method
#[test]
fn test_csm_init_shell_hook() -> Result<(), Error> {
    let cases = vec![
        // (shell, suffix, env_var, expected_csm_shell)
        #[cfg(feature = "__test_bash")]
        ("bash", ".sh", "$_CSM_SHELL", "bash"),
        #[cfg(feature = "__test_fish")]
        ("fish", ".sh", "$_CSM_SHELL", "fish"),
        #[cfg(feature = "__test_powershell")]
        ("pwsh", ".ps1", "$env:_CSM_SHELL", "powershell"),
        #[cfg(feature = "__test_zsh")]
        ("zsh", ".sh", "$_CSM_SHELL", "zsh"),
    ];
    let csm = common::Csm::new()?;
    for (shell, suffix, env_var, expected_csm_shell) in cases {
        let supported_shell = csm::shell::SupportedShell::from_str(shell, false).unwrap();
        let config = csm::shell::ShellConfiguration::from_supported_shell(&supported_shell);
        let script = format!("{}; echo {}", config.init_command, env_var);
        csm.run_script(shell, &script, suffix)?
            .assert()
            .success()
            .stdout(predicate::eq(format!(
                "{}{}",
                expected_csm_shell,
                common::EOL
            )));
    }
    Ok(())
}

/// Test shell auto-detect
#[test]
fn test_csm_init_shell_autodetect() -> Result<(), Error> {
    let cases = vec![
        // (cmd, suffix, rendered shell name)
        #[cfg(feature = "__test_bash")]
        ("bash", ".sh", "bash"),
        #[cfg(feature = "__test_fish")]
        ("fish", ".sh", "fish"),
        #[cfg(feature = "__test_powershell")]
        ("pwsh", ".ps1", "powershell"),
        #[cfg(feature = "__test_zsh")]
        ("zsh", ".sh", "zsh"),
    ];
    let csm = common::Csm::new()?;
    for (shell, suffix, expected) in cases {
        csm.run_script(shell, "csm init", suffix)?
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(
                "We think you are using {}",
                expected
            )));
    }
    Ok(())
}
