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
