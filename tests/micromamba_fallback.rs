mod common;

use common::Error;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;
use which::which;

/// Return the directory containing `micromamba` (linux) or
/// `micromamba.exe` (windows). Used for setting $PATH in tests below.
fn micromamba_path_dir() -> Result<PathBuf, Error> {
    let micromamba = which("micromamba")?;
    micromamba.parent().map(|p| p.to_path_buf()).ok_or_else(|| {
        Error::IO(io::Error::new(
            io::ErrorKind::NotFound,
            "No parent directory",
        ))
    })
}

/// Test micromamba found in $PATH
#[test]
fn test_micromamba_in_path_success() -> Result<(), Error> {
    let temp_dir = TempDir::new()?;

    let mut cmd = cargo_bin_cmd!();
    cmd.env("PATH", micromamba_path_dir()?)
        .env("HOME", temp_dir.path()) // Avoid reading real .csmrc
        .env("USERPROFILE", temp_dir.path()) // And on Windows...
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure() // no robotmk-env.yaml, so micromamba fails
        .stderr(predicate::str::contains("Ran micromamba found in $PATH"));
    Ok(())
}

/// Test micromamba found in $PATH but fails to execute
#[test]
fn test_micromamba_in_path_cannot_be_run() -> Result<(), Error> {
    let temp_dir = TempDir::new()?;

    // Just create a file named micromamba that is not executable
    let micromamba_path = if cfg!(windows) {
        temp_dir.path().join("micromamba.exe")
    } else {
        temp_dir.path().join("micromamba")
    };
    fs::write(&micromamba_path, "not an executable").unwrap();

    let mut cmd = cargo_bin_cmd!();
    cmd.env("PATH", temp_dir.path())
        .env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
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
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir)?;

    let config = format!(
        "cache_dir: {}\ndownload_micromamba: false",
        cache_dir.to_string_lossy()
    );
    common::write_csmrc(temp_dir.path(), &config)?;

    // Copy micromamba binary to cache dir
    let binary_name = if cfg!(windows) {
        "micromamba.exe"
    } else {
        "micromamba"
    };
    fs::copy(which("micromamba")?, cache_dir.join(binary_name))?;

    let mut cmd = cargo_bin_cmd!();
    cmd.env("PATH", temp_dir.path()) // Something without micromamba
        .env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
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
    let temp_dir = TempDir::new()?;
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir)?;

    let config = format!(
        "cache_dir: {}\ndownload_micromamba: false",
        cache_dir.to_string_lossy()
    );
    common::write_csmrc(temp_dir.path(), &config)?;

    let empty_path_dir = temp_dir.path().join("empty_path");
    fs::create_dir_all(&empty_path_dir)?;

    let mut cmd = cargo_bin_cmd!();
    cmd.env("PATH", empty_path_dir)
        .env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
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
    let temp_dir = TempDir::new()?;

    let mut cmd = cargo_bin_cmd!();
    cmd.env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .args(&["--verbose", "--noop", "env", "create", "--name", "test-env"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Would run:"));
    Ok(())
}
