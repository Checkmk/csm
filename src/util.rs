use std::env;

pub fn homedir() -> Result<String, env::VarError> {
    env::var("HOME").or_else(|_| env::var("USERPROFILE"))
}
