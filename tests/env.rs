mod common;

use common::Error;

use predicates::prelude::*;

/// Create an environment with `csm env create`.
fn csm_env_create(csm: &mut common::Csm, name: &str) -> Result<(), Error> {
    csm.command()
        .args(["env", "create", "-n", name])
        .current_dir(common::tests_dir().join("micromamba-minimal"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Done."));
    Ok(())
}

/// Test `csm env create`
#[test]
fn test_csm_env_create() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_create")?;

    // Ensure the post build command ran, it should create a file called
    // post_build in the home directory.
    let file_path = csm.home_dir.path().join("post_build");
    assert!(file_path.exists());
    Ok(())
}

/// Create an environment and then test that it shows in `csm env list`
#[test]
fn test_csm_env_list() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_list")?;
    csm.command()
        .args(["env", "list"])
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
        .args(["env", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("populated config files : "));
    Ok(())
}

/// Test `csm env run`
#[test]
fn test_csm_env_run() -> Result<(), Error> {
    // Be able to test both with and without the `--` separator
    fn args(with_double_dash: bool) -> Vec<&'static str> {
        let always_args = vec!["env", "run", "-n", "test_csm_env_run"];
        let mut args = always_args.clone();
        if with_double_dash {
            args.push("--");
        }
        if cfg!(windows) {
            args.extend(&["powershell", "-Command", "echo $env:CONDA_PROMPT_MODIFIER"]);
        } else {
            args.extend(&["sh", "-c", "echo $CONDA_PROMPT_MODIFIER"]);
        }
        args
    }

    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_run")?;
    for with_double_dash in [false, true] {
        csm.command()
            .args(args(with_double_dash))
            .assert()
            .success()
            .stdout(predicate::str::contains("(test_csm_env_run)"));
    }
    Ok(())
}

/// Test that running a command like `csm env info` generates a `.mambarc` in
/// the user's home directory.
#[test]
fn test_csm_creates_mambarc() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command().args(["env", "info"]).assert().success();
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
        .stderr(predicate::str::contains("found in \"robotmk-env.yaml\""));

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
        .args(["env", "activate"])
        .env_clear()
        .assert()
        .failure()
        .stderr(predicate::str::contains("See 'csm init' for information"));
    Ok(())
}

/// Activate an environment and call something in it, using bash.
/// Then deactivate it and ensure that thing was removed from PATH.
#[cfg(feature = "__test_bash")]
#[test]
fn csm_env_activate_deactivate_bash() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    let _ = csm_env_create(&mut csm, "csm_env_activate_deactivate_bash");

    // activate
    csm.run_script(
        "bash",
        "eval \"$(csm init bash --code)\" &&\
             csm env activate -n csm_env_activate_deactivate_bash &&\
             robot --version",
        ".sh",
    )?
    .assert()
    .code(251)
    .stdout(predicate::str::is_match("^Robot Framework")?);

    // deactivate
    csm.run_script(
        "bash",
        "eval \"$(csm init bash --code)\" &&\
             csm env activate -n csm_env_activate_deactivate_bash &&\
             csm env deactivate &&\
             robot --version",
        ".sh",
    )?
    .assert()
    .failure()
    .stderr(predicate::str::contains("command not found"));
    Ok(())
}

/// Activate an environment and call something in it, using powershell.
/// Then deactivate it and ensure that thing was removed from PATH.
#[cfg(feature = "__test_powershell")]
#[test]
fn csm_env_activate_deactivate_powershell() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    let _ = csm_env_create(&mut csm, "csm_env_activate_deactivate_powershell");

    // activate
    csm.run_script(
        "pwsh",
        "csm init powershell --code | Out-String | Invoke-Expression &&\
             csm env activate -n csm_env_activate_deactivate_powershell &&\
             robot --version",
        ".ps1",
    )?
    .assert()
    .stdout(predicate::str::is_match("^Robot Framework")?);

    // deactivate
    csm.run_script(
        "pwsh",
        "csm init powershell --code | Out-String | Invoke-Expression &&\
             csm env activate -n csm_env_activate_deactivate_powershell &&\
             csm env deactivate &&\
             robot --version",
        ".ps1",
    )?
    .assert()
    .stderr(predicate::str::contains(
        "not recognized as a name of a cmdlet",
    ));
    Ok(())
}

/// Create an environment and then test that it can be packed and unpacked with
/// `csm env pack` and `csm env unpack`.
#[test]
fn test_csm_env_pack_unpack() -> Result<(), Error> {
    let mut csm = common::Csm::new()?;
    csm_env_create(&mut csm, "test_csm_env_pack")?;

    let mut args = vec![
        "env",
        "pack",
        "-n",
        "test_csm_env_pack",
        "-o",
        "packed.tar.gz",
    ];

    // If conda-pack isn't installed, we give a useful error
    csm.command()
        .args(&args)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "conda-pack was not found in the environment.",
        ));

    csm.command()
        .args([
            "env",
            "run",
            "-n",
            "test_csm_env_pack",
            "pip",
            "install",
            "conda-pack==0.8.1",
        ])
        .assert()
        .success();

    csm.command()
        .args(&args)
        .assert()
        .success()
        .stdout(predicate::str::contains("100% Completed"));

    // Now the file has been created, so we can't do it again with the same name
    // without --force
    csm.command()
        .args(&args)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "File 'packed.tar.gz' already exists",
        ));

    // But forcing should work.
    args.push("--force");
    csm.command()
        .args(&args)
        .assert()
        .success()
        .stdout(predicate::str::contains("100% Completed"));

    csm.command()
        .args(["env", "unpack", "-n", "unpacked", "packed.tar.gz"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Successfully unpacked environment",
        ));

    let (shell, cmd) = if cfg!(windows) {
        ("powershell", "echo $env:CONDA_PROMPT_MODIFIER")
    } else {
        ("sh", "echo $CONDA_PROMPT_MODIFIER")
    };

    csm.command()
        .args(["env", "run", "-n", "unpacked", "--", shell, "-c", cmd])
        .assert()
        .success()
        .stdout(predicate::str::contains("(unpacked)"));

    Ok(())
}

/// Test `csm env unpack` environment name detection
#[test]
fn test_csm_env_unpack_detect_name() -> Result<(), Error> {
    let csm = common::Csm::new()?;

    for path in ["/path/to/my_env.tar.gz", "my_env.tgz"] {
        csm.command()
            .args(["-n", "env", "unpack", path])
            .assert()
            .failure() // with -n, it won't be able to resolve paths, so it'll fail
            .stderr(predicate::str::contains(
                "Using 'my_env' as environment name",
            ));
    }

    Ok(())
}

/// Test handling of robotmk-setup.yaml and alternatives.
#[test]
fn csm_env_create_with_setup() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(["-n", "env", "create", "--setup-file", "doesnotexist"])
        .env_clear()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Explicit --setup-file"))
        .stderr(predicate::str::contains("does not exist"));

    std::fs::File::create(csm.home_dir.path().join("testsetup.yaml"))?;

    csm.command()
        .args(["-nv", "env", "create", "--setup-file", "testsetup.yaml"])
        .env_clear()
        .assert()
        .success()
        .stderr(predicate::str::contains("Read setup file 'testsetup.yaml'"));

    csm.command()
        .args([
            "-nv",
            "env",
            "create",
            "--setup-file",
            &csm.home_dir.path().to_string_lossy(),
        ])
        .env_clear()
        .assert()
        .success()
        .stderr(predicate::str::contains("but could not read it"));

    Ok(())
}
