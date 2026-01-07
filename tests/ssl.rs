mod common;

use common::Error;

use assert_cmd::cmd::Command;
use predicates::prelude::*;

/// Test `csm env create --ssl-verify=false` / ssl_verify: false
#[test]
fn test_csm_ssl_verify_false() -> Result<(), Error> {
    fn assert_env_vars(mut command: Command) {
        command
            .assert()
            .success()
            .stderr(predicate::str::contains("\"CURL_CA_BUNDLE\": \"\""))
            .stderr(predicate::str::contains("\"PIP_CERT\": \"\""))
            .stderr(predicate::str::contains("\"REQUESTS_CA_BUNDLE\": \"\""))
            .stderr(predicate::str::contains(
                "\"PIP_INDEX_URL\": \"https://pypi.org/simple\"",
            ))
            .stderr(predicate::str::contains(
                "\"PIP_TRUSTED_HOST\": \"pypi.org files.pythonhosted.org pypi.pythonhosted.org\"",
            ))
            .stderr(predicate::str::contains("--ssl-verify=<false>"));
    }

    let csm = common::Csm::new()?;
    let mut command = csm.command();
    command.args(vec![
        "-nv",
        "env",
        "create",
        "-n",
        "foo",
        "--ssl-verify=false",
    ]);
    assert_env_vars(command);

    csm.write_csmrc("env_create:\r\n  ssl_verify: false")?;
    command = csm.command();
    command.args(vec!["-nv", "env", "create"]);
    assert_env_vars(command);

    Ok(())
}

/// Test `csm --ssl-verify=true` (should set no env vars) - also the default
#[test]
fn test_csm_ssl_verify_true() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    for arg_list in [
        vec!["-nv", "env", "create", "--ssl-verify=true"], // explicit
        vec!["-nv", "env", "list"],                        // default
    ] {
        csm.command()
            .args(arg_list)
            .assert()
            .success()
            .stderr(predicate::str::contains("CURL_CA_BUNDLE").not())
            .stderr(predicate::str::contains("PIP_CERT").not())
            .stderr(predicate::str::contains("REQUESTS_CA_BUNDLE").not())
            .stderr(predicate::str::contains("PIP_INDEX_URL").not())
            .stderr(predicate::str::contains("PIP_TRUSTED_HOST").not())
            .stderr(predicate::str::contains("--ssl-verify=<false>").not());
    }

    Ok(())
}

/// Test `csm --ssl-verify=/tmp/hey` / ssl_bundle: /tmp/hey
#[test]
fn test_csm_ssl_verify_custom_bundle() -> Result<(), Error> {
    fn assert_env_vars(mut command: Command) {
        command
            .assert()
            .success()
            .stderr(predicate::str::contains("\"CURL_CA_BUNDLE\": \"/tmp/hey\""))
            .stderr(predicate::str::contains("\"PIP_CERT\": \"/tmp/hey\""))
            .stderr(predicate::str::contains(
                "\"REQUESTS_CA_BUNDLE\": \"/tmp/hey\"",
            ))
            .stderr(predicate::str::contains("PIP_INDEX_URL").not())
            .stderr(predicate::str::contains("PIP_TRUSTED_HOST").not())
            .stderr(predicate::str::contains("--ssl-verify=<false>").not());
    }

    let csm = common::Csm::new()?;
    let mut command = csm.command();
    command.args(vec!["-nv", "env", "create", "--ssl-verify=/tmp/hey"]);
    assert_env_vars(command);

    csm.write_csmrc("env_create:\r\n  ssl_bundle: /tmp/hey")?;
    command = csm.command();
    command.args(vec!["-nv", "env", "create"]);
    assert_env_vars(command);

    Ok(())
}

/// Test `csm --ssl-no-revoke` / ssl_no_revoke: true
#[test]
fn test_csm_ssl_no_revoke() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.command()
        .args(vec!["-nv", "env", "create", "--ssl-no-revoke"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "\"MAMBA_SSL_NO_REVOKE\": \"true\"",
        ));

    csm.command()
        .args(vec!["-nv", "env", "create"])
        .assert()
        .success()
        .stderr(predicate::str::contains("MAMBA_SSL_NO_REVOKE").not());

    csm.write_csmrc("env_create:\r\n  ssl_no_revoke: true")?;
    csm.command()
        .args(vec!["-nv", "env", "create"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "\"MAMBA_SSL_NO_REVOKE\": \"true\"",
        ));

    Ok(())
}

/// Test config validation: ssl_verify: false + ssl_bundle => reject
#[test]
fn test_csm_ssl_verify_false_and_ssl_bundle() -> Result<(), Error> {
    let csm = common::Csm::new()?;
    csm.write_csmrc("env_create:\r\n  ssl_verify: false\r\n  ssl_bundle: foo.pem")?;
    csm.command()
        .args(vec!["-nv", "env", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "SSL/TLS verification was disabled",
        ));
    Ok(())
}
