# csm - Checkmk synthetic monitoring

A command-line tool to manage Robotmk environments.

After installing (see the 'Releases' section on the right side of the GitHub
repository page), run `./csm help` for usage information.

There is a shell hook which can be installed to help integrate `csm` into your
shell. After `csm` is installed (ideally somewhere in your `PATH`), see
`csm init` for instructions on how to set up the hook.

## Configuration: `~/.csmrc`

You can optionally create a file, `~/.csmrc` (`%UserProfile%\.csmrc` on Windows)
to override certain defaults. This is a YAML file with the following keys
available:

* `mamba_root_prefix` - A string which sets where the Mamba environment(s) will
  be created on disk. By default, this is left up to `micromamba` and its
  default root prefix is used.

* `cache_dir` - A string path which is used as the cache directory. Currently,
  this is used for storing the `micromamba` binary if it is downloaded by `csm`
  (see next option).

* `download_micromamba` - A boolean, determines whether or not `csm` should try
  to download `micromamba` if it was not found in `$PATH`. This is mostly useful
  for testing, but could be useful if you wish to ensure `csm` never downloads
  `micromamba` even if the one in `$PATH` should disappear.

* `skip_longpaths_check` - A boolean, which, if true, disables checking for
  LongPaths support on Windows. By default when `csm.exe` runs, it will check
  for LongPaths support. If not already enabled, it will try to enable it. If
  this fails (for example, because the user running `csm.exe` is not an
  administrator), `csm.exe` will prompt for confirmation before continuing.
  Setting this configuration option to true, disables the check entirely. This
  option does nothing on non-Windows systems.

* **`env_create` section** - for options affecting the `csm env create`
  subcommand.

  * `ssl_verify` - A boolean, which, if true (default) enables SSL/TLS
    certificate verification. If false, verification is disabled when calling
    `micromamba` and when using `pip`. Setting this to false has the effect of
    setting the following environment variables when `csm` calls `micromamba`:

    - `CURL_CA_BUNDLE=""`
    - `PIP_CERT=""`
    - `PIP_INDEX_URL="https://pypi.org/simple"`
    - `PIP_TRUSTED_HOST="pypi.org files.pythonhosted.org pypi.pythonhosted.org"`
    - `REQUESTS_CA_BUNDLE=""`

    This setting (and `ssl_bundle` below) can be overridden with the
    `--ssl-verify` command-line argument to `csm env create`.

  * `ssl_bundle` - A string which configures the SSL/TLS certificate bundle to
    use when verifying certificates. Only taken into account when `ssl_verify`
    is true. Setting this has the effect of setting the following environment
    variables when `csm` calls `micromamba`:

    - `CURL_CA_BUNDLE="<value>"`
    - `PIP_CERT="<value>"`
    - `REQUESTS_CA_BUNDLE="<value>"`

  * `ssl_no_revoke` - A boolean which instructs `micromamba` to not check
    SSL/TLS certificate revokation. Setting this has the effect of setting the
    following environment variable when `csm` calls `micromamba`:

    - `MAMBA_SSL_NO_REVOKE="true"`

    This setting can be overridden with the `--ssl-no-revoke` command-line
    switch to `csm env create`.
