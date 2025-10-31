use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use which::which;

/// Return the directory containing `micromamba` (linux) or
/// `micromamba.exe` (windows). Used for setting $PATH in tests below.
fn micromamba_path_dir() -> PathBuf {
    let micromamba = which("micromamba").unwrap();
    micromamba.parent().unwrap().to_path_buf()
}

/// Helper to create a csmrc config file for testing with downloads disabled
fn create_test_csmrc(home_dir: &Path, cache_dir: &Path) {
    let csmrc_content = format!(
        "cache_dir: {}\n\
         download_micromamba: false\n",
        cache_dir.display()
    );
    fs::write(home_dir.join(".csmrc"), csmrc_content).unwrap();
}

/// Test micromamba found in $PATH
#[test]
fn test_micromamba_in_path_success() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!();
    cmd.env("PATH", micromamba_path_dir())
        .env("HOME", temp_dir.path()) // Avoid reading real .csmrc
        .env("USERPROFILE", temp_dir.path()) // And on Windows...
        .args(&["--verbose", "env", "create", "--name", "test-env"])
        .assert()
        .failure() // no robotmk-env.yaml, so micromamba fails
        .stderr(predicate::str::contains("Ran micromamba found in $PATH"));
}

/// Test micromamba found in $PATH but fails to execute
#[test]
fn test_micromamba_in_path_cannot_be_run() {
    let temp_dir = TempDir::new().unwrap();

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
}

/// micromamba not in $PATH, but cached version exists and works
#[test]
fn test_micromamba_fallback_to_cache_success() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    // Copy micromamba binary to cache dir
    let binary_name = if cfg!(windows) {
        "micromamba.exe"
    } else {
        "micromamba"
    };
    fs::copy(which("micromamba").unwrap(), cache_dir.join(binary_name)).unwrap();

    create_test_csmrc(temp_dir.path(), &cache_dir);

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
}

/// micromamba not in $PATH and not cached, download would be attempted but is disabled
#[test]
fn test_micromamba_fallback_to_download() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    create_test_csmrc(temp_dir.path(), &cache_dir);

    let empty_path_dir = temp_dir.path().join("empty_path");
    fs::create_dir_all(&empty_path_dir).unwrap();

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
}

/// noop mode doesn't actually execute anything
#[test]
fn test_noop_mode() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!();
    cmd.env("HOME", temp_dir.path())
        .env("USERPROFILE", temp_dir.path())
        .args(&["--verbose", "--noop", "env", "create", "--name", "test-env"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Would run:"));
}
