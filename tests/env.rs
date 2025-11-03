mod common;

use common::Error;

use predicates::prelude::*;

/// Create an environment with `csm env create`.
fn csm_env_create(csm: &mut common::Csm, name: &str) -> Result<(), Error> {
    csm.command()
        .args(&["env", "create", "-n", name])
        .current_dir(common::tests_dir().join("micromamba-minimal"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Transaction finished"));
    Ok(())
}

/// Test `csm env create`
#[test]
fn test_csm_env_create() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_create")
}

/// Create an environment and then test that it shows in `csm env list`
#[test]
fn test_csm_env_list() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_list")?;
    csm.command()
        .args(&["env", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_match("Name.*Active.*Path")?)
        .stdout(predicate::str::contains("test_csm_env_list"));
    Ok(())
}

/// Test `csm env info`
#[test]
fn test_csm_env_info() -> Result<(), Error> {
    common::Csm::new()?
        .command()
        .args(&["env", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("populated config files : "));
    Ok(())
}

/// Test that running a command like `csm env info` generates a `.mambarc` in
/// the user's home directory.
#[test]
fn test_csm_creates_mambarc() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command().args(&["env", "info"]).assert().success();
    let mambarc_path = csm.home_dir.path().join(".mambarc");
    Ok(std::fs::read_to_string(mambarc_path).map(|_| ())?)
}
