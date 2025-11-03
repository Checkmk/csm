mod common;

use common::Error;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Test `csm env list`
#[test]
fn test_csm_env_list() -> Result<(), Error> {
    cargo_bin_cmd!()
        .args(&["env", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_match("Name.*Active.*Path")?);
    Ok(())
}
