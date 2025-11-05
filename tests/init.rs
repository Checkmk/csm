mod common;

use common::Error;

use predicates::prelude::*;

/// Test `csm init` shell auto-detect.
///
/// We do not assume the test runner to be using a particular shell other than
/// one that is supported.
#[test]
fn test_csm_init() -> Result<(), Error> {
    common::Csm::new()?
        .command()
        .args(&["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("We think you are using"));
    Ok(())
}

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
