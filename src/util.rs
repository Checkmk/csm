use std::env;
use std::path::PathBuf;

pub fn homedir() -> Option<PathBuf> {
    env::var("HOME").ok().or_else(|| env::var("USERPROFILE").ok()).map(PathBuf::from)
}
