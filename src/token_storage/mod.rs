extern crate yup_oauth2;
use std::io;
use std::path::{PathBuf};
use futures_locks::Mutex;
use async_trait::async_trait;
use yup_oauth2::storage::{
    TokenStorage,
    TokenInfo,
};

mod json_tokens;
mod keychain;

use json_tokens::{JSONTokens, ScopeSet, JSONToken};
use keychain::{Keychain};
use crate::fs::{open_writeable_file};

type TokenCollection = Mutex<JSONTokens>;

#[derive(Clone)]
pub struct KeychainStorage {
    tokens: TokenCollection,
    keychain: Keychain,
    filename: Option<PathBuf>,
}

impl KeychainStorage {
    pub async fn new(filename: Option<PathBuf>) -> Result<Self, io::Error> {
        let keychain = Keychain::new();

        Ok(KeychainStorage {
            keychain,
            filename: None,
            tokens: Mutex::new(JSONTokens::new()),
        })
    }

    pub async fn hydrate_from_file(&mut self, filename: PathBuf) -> Result<Self, io::Error> {
        let fs_tokens = match JSONTokens::load_from_file(&filename).await {
            Ok(tokens) => tokens,
            Err(e) if e.kind() == io::ErrorKind::NotFound => JSONTokens::new(),
            Err(e) => return Err(e),
        };

        let mut tokens = self.tokens.lock().await;

        for json_token in fs_tokens.iter() {
            let scope_set = ScopeSet::from(&json_token.scopes);
            tokens.set(scope_set, json_token.token.clone()).unwrap();
        }

        let _ = &self.update_keychain();

        Ok(self.to_owned())
    }

    pub async fn update_keychain(&self) -> Result<Self, keyring::Error> {
        let tokens = self.tokens.lock().await;

        for json_token in tokens.iter() {
            let _ = &self.keychain.update_entry(
                json_token.hash,
                json_token
            )?;
        }

        Ok(self.to_owned())
    }

    async fn set_token<T>(
        &self,
        scopes: ScopeSet<'_, T>,
        token: TokenInfo,
    ) -> anyhow::Result<(), anyhow::Error>
    where
        T: AsRef<str>,
    {
        let filename = match &self.filename {
            Some(fname) => fname,
            None => return Ok(())
        };

        use tokio::io::AsyncWriteExt;
        let json = {
            use std::ops::Deref;
            let mut lock = self.tokens.lock().await;
            lock.set(scopes, token)?;
            serde_json::to_string(lock.deref())
                .map_err(
                    |e| io::Error::new(io::ErrorKind::InvalidData, e)
                )?
        };

        let mut f = open_writeable_file(&filename).await?;
        f.write_all(json.as_bytes()).await?;
        Ok(())
    }

    async fn get_token<T>(
        &self,
        scopes: ScopeSet<'_, T>
    ) -> Option<TokenInfo>
    where
        T: AsRef<str>,
    {
        self.tokens.lock().await.get(scopes)
    }
}

#[async_trait]
impl TokenStorage for KeychainStorage {
    async fn set(
        &self,
        scopes: &[&str],
        token: TokenInfo
    ) -> anyhow::Result<()> {
        let scope_set = ScopeSet::from(scopes);
        self.set_token(scope_set, token).await
    }

    async fn get(&self, scopes: &[&str]) -> Option<TokenInfo> {
        let scope_set = ScopeSet::from(scopes);
        self.get_token(scope_set).await
    }
}