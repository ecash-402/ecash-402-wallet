use derive_more::From;
use std::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    #[from]
    WalletError(cdk::Error),
}

impl Error {
    pub fn custom(msg: &str) -> Self {
        Error::WalletError(cdk::Error::Custom(msg.to_string()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WalletError(e) => write!(f, "Wallet error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::WalletError(e) => Some(e),
        }
    }
}
