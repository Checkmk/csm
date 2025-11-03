#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Which(which::Error),
    IO(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<which::Error> for Error {
    fn from(err: which::Error) -> Self {
        Self::Which(err)
    }
}
