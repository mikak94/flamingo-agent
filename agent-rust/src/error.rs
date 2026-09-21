use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

// some are unreachable, depending on the target
#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// `op` names the failing call.
    Os {
        op: &'static str,
        code: u32,
    },
    Config(String),
    Other(String),
}

#[allow(dead_code)]
impl Error {
    pub fn os(op: &'static str, code: u32) -> Self {
        Error::Os { op, code }
    }

    pub fn last_os(op: &'static str) -> Self {
        let code = io::Error::last_os_error().raw_os_error().unwrap_or(0);
        Error::Os {
            op,
            code: code as u32,
        }
    }

    pub fn config(msg: &str) -> Self {
        Error::Config(msg.into())
    }

    pub fn other(msg: &str) -> Self {
        Error::Other(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "i/o error: {e}"),
            Error::Os { op, code } => {
                let msg = io::Error::from_raw_os_error(*code as i32);
                write!(f, "{op} failed (os error {code}): {msg}")
            }
            Error::Config(msg) => f.write_str(msg),
            Error::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
