mod common;

use common::Error;

use predicates::prelude::*;
use std::fs;
use which::which;

/// Test micromamba found in $PATH
#[test]
fn test_micromamba_in_path_success() -> Result<(), Error> {
    common::Csm::new()?
        .command()
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure() // no robotmk-env.yaml, so micromamba fails
        .stderr(predicate::str::contains("Ran micromamba found in $PATH"));
    Ok(())
}

/// Test micromamba found in $PATH but fails to execute
#[test]
fn test_micromamba_in_path_cannot_be_run() -> Result<(), Error> {
    let csm = common::Csm::new()?;

    // Just create a file named micromamba that is not executable
    let micromamba_path = if cfg!(windows) {
        csm.home_dir.path().join("micromamba.exe")
    } else {
        csm.home_dir.path().join("micromamba")
    };
    fs::write(&micromamba_path, "not an executable").unwrap();

    let mut cmd = csm.command();
    cmd.env("PATH", csm.home_dir.path())
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "micromamba found in $PATH could not be run, aborting",
        ));
    Ok(())
}

/// micromamba not in $PATH, but cached version exists and works
#[test]
fn test_micromamba_fallback_to_cache_success() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    let cache_dir = csm.home_dir.path().join("cache");
    fs::create_dir_all(&cache_dir)?;

    let config = format!(
        "cache_dir: {}\ndownload_micromamba: false",
        cache_dir.to_string_lossy()
    );
    csm.write_csmrc(&config)?;

    // Copy micromamba binary to cache dir
    let binary_name = if cfg!(windows) {
        "micromamba.exe"
    } else {
        "micromamba"
    };
    fs::copy(which("micromamba")?, cache_dir.join(binary_name))?;

    let mut cmd = csm.command();
    cmd.env("PATH", csm.home_dir.path()) // Something without micromamba
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure() // no robotmk-env.yaml, so micromamba fails
        .stderr(predicate::str::contains(
            "micromamba not found in $PATH, falling back to cache",
        ))
        .stderr(predicate::str::contains("Ran downloaded/cached micromamba"))
        .stderr(predicate::str::contains("Wanted to download micromamba").not());
    Ok(())
}

/// micromamba not in $PATH and not cached, download would be attempted but is disabled
#[test]
fn test_micromamba_fallback_to_download() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    let cache_dir = csm.home_dir.path().join("cache");
    fs::create_dir_all(&cache_dir)?;

    let config = format!(
        "cache_dir: {}\ndownload_micromamba: false",
        cache_dir.to_string_lossy()
    );
    csm.write_csmrc(&config)?;

    let mut cmd = csm.command();
    cmd.env("PATH", csm.home_dir.path()) // Something without micromamba
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure() // Should fail because downloads are disabled
        .stderr(predicate::str::contains(
            "micromamba not found in $PATH, falling back to cache",
        ))
        .stderr(predicate::str::contains("Wanted to download micromamba"));
    Ok(())
}

/// noop mode doesn't actually execute anything
#[test]
fn test_noop_mode() -> Result<(), Error> {
    common::Csm::new()?
        .command()
        .args(&["--verbose", "--noop", "env", "create", "--name", "test-env"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Would run:"));
    Ok(())
}
