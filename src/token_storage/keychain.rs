extern crate keyring;
use std::io;
use keyring::{ Entry, credential::PlatformCredential };
use super::json_tokens::{ JSONToken, ScopeHash };

const SERVICE_NAME: &str = "remote_conf_sync";

#[derive(Clone)]
pub struct Keychain {
    user: String,
    service: String,
}

impl Keychain {
    pub fn new() -> Keychain {
        whoami::username();

        Keychain {
            user: whoami::username(),
            service: SERVICE_NAME.into(),
        }
    }

    pub fn update_entry(
        &self,
        scope_hash: ScopeHash,
        token: &JSONToken
    ) -> Result<Entry, keyring::Error> {
        let cred_key = scope_hash.0.to_string();
        let entry = Entry::new_with_target(
            &cred_key,
            &self.service,
            &self.user
        );

        let json = {
            serde_json::to_string(token)
                .map_err(
                    |e| io::Error::new(io::ErrorKind::InvalidData, e)
                ).unwrap()
        };

        let existing = match entry.get_password_and_credential() {
            Ok((existing_key, creds)) => {},
            Err(e) => match e {
                keyring::Error::NoEntry => {
                    entry.set_password(&json)?;
                },
                _ => {
                    panic!("{}: {}", "Error occurred while retrieving keychain entires", e);
                }
            }
        };

        Ok(entry)
    }
}