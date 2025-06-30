use derive_more::From;

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
