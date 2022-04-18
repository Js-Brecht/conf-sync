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

use json_tokens::{JSONTokens, ScopeSet};
use crate::fs::{open_writeable_file};

// fn get_scope_id(scopes: &[&str]) {
//     let str_scopes: Vec<&str> = scopes
//         .iter()
//         .map(|scope| scope.as_ref())
//         .sorted()
//         .unique()
//         .collect();
// }

#[derive(Clone)]
pub struct KeychainStorage {
    tokens: Mutex<JSONTokens>,
    filename: PathBuf,
}

impl KeychainStorage {
    pub async fn new(filename: PathBuf) -> Result<Self, io::Error> {
        let tokens = match JSONTokens::load_from_file(&filename).await {
            Ok(tokens) => tokens,
            Err(e) if e.kind() == io::ErrorKind::NotFound => JSONTokens::new(),
            Err(e) => return Err(e),
        };

        Ok(KeychainStorage {
            tokens: Mutex::new(tokens),
            filename,
        })
    }

    async fn set_token<T>(
        &self,
        scopes: ScopeSet<'_, T>,
        token: TokenInfo,
    ) -> anyhow::Result<(), anyhow::Error>
    where
        T: AsRef<str>,
    {
        use tokio::io::AsyncWriteExt;
        let json = {
            use std::ops::Deref;
            let mut lock = self.tokens.lock().await;
            lock.set(scopes, token)?;
            serde_json::to_string(lock.deref())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        };
        let mut f = open_writeable_file(&self.filename).await?;
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