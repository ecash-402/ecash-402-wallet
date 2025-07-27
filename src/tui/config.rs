use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub name: String,
    pub nsec: String,
    pub mints: Vec<String>,
    pub relays: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub wallets: Vec<WalletConfig>,
    pub active_wallet: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| crate::error::Error::custom("Could not find home directory"))?;
        Ok(home.join(".config").join("nip60").join("tui.yaml"))
    }

    pub fn add_wallet(&mut self, wallet: WalletConfig) {
        if self.wallets.is_empty() {
            self.active_wallet = Some(wallet.name.clone());
        }
        self.wallets.push(wallet);
    }

    pub fn remove_wallet(&mut self, name: &str) -> bool {
        if let Some(index) = self.wallets.iter().position(|w| w.name == name) {
            self.wallets.remove(index);
            if self.active_wallet.as_ref() == Some(&name.to_string()) {
                self.active_wallet = self.wallets.first().map(|w| w.name.clone());
            }
            true
        } else {
            false
        }
    }

    pub fn get_active_wallet(&self) -> Option<&WalletConfig> {
        self.active_wallet
            .as_ref()
            .and_then(|name| self.wallets.iter().find(|w| w.name == *name))
    }

    pub fn set_active_wallet(&mut self, name: &str) -> bool {
        if self.wallets.iter().any(|w| w.name == name) {
            self.active_wallet = Some(name.to_string());
            true
        } else {
            false
        }
    }
}

impl WalletConfig {
    pub fn new(name: String, nsec: String) -> Self {
        Self {
            name,
            nsec,
            mints: vec![
                "https://ecashmint.otrta.me".to_string(),
                "https://mint.minibits.cash/Bitcoin".to_string(),
            ],
            relays: vec![
                "wss://relay.primal.net".to_string(),
                "wss://relay.damus.io".to_string(),
                "wss://nostr.oxtr.dev".to_string(),
                "wss://nostr.mom".to_string(),
            ],
            active: true,
        }
    }
}
