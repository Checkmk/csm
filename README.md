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
