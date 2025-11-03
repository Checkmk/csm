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
