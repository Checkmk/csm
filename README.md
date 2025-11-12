# csm - Checkmk synthetic monitoring

(Under active development, not yet usable.)

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

* `download_micromamba` - A boolean, determines whether or not we should try to
  download `micromamba` if it was not found in `$PATH`. This is mostly useful
  for testing, but could be useful if you wish to ensure `csm` never downloads
  `micromamba` even if the one in `$PATH` should disappear.

* `skip_longpaths_check` - A boolean, which, if true, disables checking for
  LongPaths support on Windows. By default when `csm.exe` runs, it will check
  for LongPaths support. If not already enabled, it will try to enable it. If
  this fails (for example, because the user running `csm.exe` is not an
  administrator), `csm.exe` will prompt for confirmation before continuing.
  Setting this configuration option to true, disables the check entirely.
