mod common;

use common::Error;

use assert_cmd::cargo;
use assert_cmd::cmd::Command;
use predicates::prelude::*;
use which::which;

/// Test `csm init <shell>`
#[test]
fn test_csm_init_shell() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(&["init", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">> ~/.bashrc"));

    csm.command()
        .args(&["init", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">> ~/.zshrc"));

    csm.command()
        .args(&["init", "unsupported"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values:"));
    Ok(())
}

/// Test `csm init <shell> --code`
#[test]
fn test_csm_init_shell_code() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(&["init", "bash", "--code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPREPLY"))
        .stdout(predicate::str::contains("complete -F"));

    csm.command()
        .args(&["init", "zsh", "--code"])
        .assert()
        .success()
        .stdout(predicate::str::contains("compdef"));

    csm.command()
        .args(&["init", "unsupported", "--code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values:"));
    Ok(())
}

/// Test `csm init bash --code`
#[cfg(feature = "__test_bash")]
#[test]
fn test_csm_init_shell_hook_bash() -> Result<(), Error> {
    let csm = cargo::cargo_bin!().to_string_lossy().replace("\\", "/");
    // Need to specify a full path here because otherwise on Windows systems
    // with WSL but no distro installed (like GHA runners), just using "bash"
    // here will try to spawn bash from System32 (and modifying $PATH does not
    // fix it, Windows CreateProcess() will prefer System32 regardless).
    Command::new(which("bash")?)
        .arg("-c")
        .arg(format!(
            "eval \"$({} init bash --code)\" && echo $_CSM_SHELL",
            csm
        ))
        .env_clear()
        .assert()
        .success()
        .stdout(predicate::eq("bash\n"));
    Ok(())
}

/// Test `csm init fish --code`
#[cfg(feature = "__test_fish")]
#[test]
fn test_csm_init_shell_hook_fish() -> Result<(), Error> {
    let csm = cargo::cargo_bin!().to_string_lossy();
    Command::new("fish")
        .arg("-c")
        .arg(format!(
            "{} init fish --code | source && echo $_CSM_SHELL",
            csm
        ))
        .env_clear()
        .assert()
        .success()
        .stdout(predicate::eq("fish\n"));
    Ok(())
}

/// Test `csm init zsh --code`
#[cfg(feature = "__test_zsh")]
#[test]
fn test_csm_init_shell_hook_zsh() -> Result<(), Error> {
    let csm = cargo::cargo_bin!().to_string_lossy();
    Command::new("zsh")
        .arg("-c")
        .arg(format!(
            "eval \"$({} init zsh --code)\" && echo $_CSM_SHELL",
            csm
        ))
        .env_clear()
        .assert()
        .success()
        .stdout(predicate::eq("bash\n")); // we share the same code for bash and zsh
    Ok(())
}

/// Test `csm init powershell --code`
#[cfg(feature = "__test_powershell")]
#[test]
fn test_csm_init_shell_hook_powershell() -> Result<(), Error> {
    let csm = cargo::cargo_bin!().to_string_lossy();
    Command::new("pwsh")
        .arg("-c")
        .arg(format!(
            "{} init powershell --code | Out-String | Invoke-Expression && echo $env:_CSM_SHELL",
            csm
        ))
        .assert()
        .success()
        .stdout(predicate::str::is_match("powershell\r?\n")?);
    Ok(())
}

/// Test shell auto-detect

#[test]
fn test_csm_init_shell_autodetect() -> Result<(), Error> {
    // Need to specify a full path here because otherwise on Windows systems
    // with WSL but no distro installed (like GHA runners), just using "bash"
    // here will try to spawn bash from System32 (and modifying $PATH does not
    // fix it, Windows CreateProcess() will prefer System32 regardless).
    let bash = which("bash")?;
    let bash = bash.to_string_lossy();
    let cases = vec![
        // (cmd, rendered shell name)
        #[cfg(feature = "__test_bash")]
        (bash.as_ref(), "bash"),
        #[cfg(feature = "__test_fish")]
        ("fish", "fish"),
        #[cfg(feature = "__test_powershell")]
        ("pwsh", "powershell"),
        #[cfg(feature = "__test_zsh")]
        ("zsh", "zsh"),
    ];
    let csm = cargo::cargo_bin!().to_string_lossy().replace("\\", "\\\\"); // Windows bash weirdness
    for shell in cases {
        Command::new(shell.0.to_string())
            .write_stdin(format!("{} init", csm))
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(
                "We think you are using {}",
                shell.1
            )));
    }
    Ok(())
}
