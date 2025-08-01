use derive_more::From;
use std::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    #[from]
    WalletError(cdk::Error),

    NotEnoughBalance(String),

    #[from]
    IoError(std::io::Error),

    #[from]
    NostrError(nostr_sdk::client::Error),

    #[from]
    NostrEventError(nostr_sdk::event::Error),

    #[from]
    SerializationError(serde_json::Error),

    #[from]
    YamlError(serde_yaml::Error),

    #[from]
    KeyError(nostr_sdk::key::Error),
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
            Error::NotEnoughBalance(e) => write!(f, "Not enough balance: {}", e),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::NostrError(e) => write!(f, "Nostr error: {}", e),
            Error::NostrEventError(e) => write!(f, "Nostr event error: {}", e),
            Error::SerializationError(e) => write!(f, "Serialization error: {}", e),
            Error::YamlError(e) => write!(f, "YAML error: {}", e),
            Error::KeyError(e) => write!(f, "Key error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::WalletError(e) => Some(e),
            Error::NotEnoughBalance(_) => None,
            Error::IoError(e) => Some(e),
            Error::NostrError(e) => Some(e),
            Error::NostrEventError(e) => Some(e),
            Error::SerializationError(e) => Some(e),
            Error::YamlError(e) => Some(e),
            Error::KeyError(e) => Some(e),
        }
    }
}
