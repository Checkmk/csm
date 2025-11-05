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

/// Test `csm env run`
#[test]
fn test_csm_env_run() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_run")?;
    let args = if cfg!(windows) {
        vec![
            "env",
            "run",
            "-n",
            "test_csm_env_run",
            "--",
            "pwsh",
            "-Command",
            "echo $env:CONDA_PROMPT_MODIFIER",
        ]
    } else {
        vec![
            "env",
            "run",
            "-n",
            "test_csm_env_run",
            "env",
            "--",
            "sh",
            "-c",
            "echo $CONDA_PROMPT_MODIFIER",
        ]
    };
    csm.command()
        .args(&args)
        .assert()
        .success()
        .stdout(predicate::str::contains("(test_csm_env_run)"));
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

/// Test determining the env name for `csm env create`.
///
/// 1. -n/--name overrides all
/// 2. if a robotmk-env.yaml is in the directory and has `name:` use it
/// 3. fall back to current directory name
#[test]
fn test_csm_env_create_env_name() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    // No --name given, but the directory has a robotmk-env.yaml
    csm.command()
        .current_dir(common::tests_dir().join("micromamba-minimal"))
        .args(vec!["-nv", "env", "create"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Using 'micromamba-minimal' as env name",
        ))
        .stderr(predicate::str::contains("found in robotmk-env.yaml"));

    // robotmk-env.yaml exists, but name overridden with --name
    csm.command()
        .current_dir(common::tests_dir().join("micromamba-minimal"))
        .args(vec!["-nv", "env", "create", "-n", "anothername"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Using 'anothername' as env name"))
        .stderr(predicate::str::contains("given by CLI argument"));

    let Some(std::path::Component::Normal(dirname)) = csm.home_dir.path().components().next_back()
    else {
        return Err("Cannot get test homedir name".to_string().into());
    };

    csm.command()
        .current_dir(csm.home_dir.path())
        .args(vec!["-nv", "env", "create"])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!(
            "Using '{}' as env name",
            dirname.to_string_lossy()
        )))
        .stderr(predicate::str::contains(
            "taken from current directory name",
        ));

    Ok(())
}

/// Test `csm env activate` with no shell hook enabled.
#[test]
fn test_csm_env_activate_no_hook() -> Result<(), Error> {
    common::Csm::new()?
        .command()
        .args(&["env", "activate"])
        .env_clear()
        .assert()
        .failure()
        .stderr(predicate::str::contains("See 'csm init' for information"));
    Ok(())
}
