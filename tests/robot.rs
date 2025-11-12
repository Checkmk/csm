mod common;

use common::Error;

use predicates::prelude::*;
use std::fs;

/// Test `csm robot new` with default/minimal template
#[test]
fn test_csm_robot_new() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(vec!["robot", "create", "myrobot"])
        .assert()
        .success();
    let path = csm.home_dir.path().join("myrobot");
    let content = fs::read_to_string(path.join("robotmk-env.yaml"))?;
    assert!(predicate::str::contains("conda-forge").eval(&content));
    Ok(())
}
