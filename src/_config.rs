// extern crate hyper;
// extern crate hyper_rustls;
use std::path::PathBuf;
use serde::{ Deserialize, Serialize };
use yup_oauth2::{ ApplicationSecret };
use directories;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
}

impl ::std::default::Default for ClientConfig {
    fn default() -> Self {
        Self {
            client_id: "".into(),
            client_secret: "".into(),
            project_id: "".into(),
            auth_uri: "".into(),
            token_uri: "".into(),
            auth_provider_x509_cert_url: "".into(),
            redirect_uris: [].to_vec(),
            scopes: [].to_vec(),
        }
    }
}

#[derive(Clone)]
pub struct Config {
    app_name: String,
    client_config: Option<ClientConfig>,
    base_dirs: directories::BaseDirs,
}

// trait ConfigManager {
//     fn new() -> Self;
//     fn config_path(&self) -> &Path;
//     fn get(&self) -> ClientConfig;
// }

impl Config {
    pub fn new() -> Self {
        Config {
            app_name: "conf-sync".into(),
            client_config: None,
            base_dirs: directories::BaseDirs::new()
        }
    }

    pub fn data_path(&self) -> PathBuf {
        self.base_dirs.data_local_dir().join(self.app_name.clone())
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dirs.config_dir().join(self.app_name.clone())
        // match self.base_dirs.config_dir() {
        //     Some(dir) => dir.to_owned(),
        //     None => {
        //         panic!("Unable to determine config directory!");
        //     }
        // }
    }

    pub fn get_client_config(&mut self) -> ClientConfig {
        match &self.client_config {
            Some(cfg) => cfg.clone(),
            None => {
                let result = confy::load::<ClientConfig>("conf-sync");
                match result {
                    Ok(content) => {
                        println!("{:#?}", content);
                        self.client_config = Some(content.clone());
                        content
                    }
                    Err(error) => {
                        panic!("Unable to read configuration: {}", error);
                    }
                }
            }
        }
    }

    pub fn get_app_secret(&mut self) -> ApplicationSecret {
        let cfg = self.get_client_config();
        ApplicationSecret {
            client_id: cfg.client_id,
            client_secret: cfg.client_secret,
            project_id: Some(cfg.project_id),
            auth_uri: cfg.auth_uri,
            token_uri: cfg.token_uri,
            redirect_uris: cfg.redirect_uris,
            auth_provider_x509_cert_url: Some(cfg.auth_provider_x509_cert_url),
            client_email: Some("".into()),
            client_x509_cert_url: Some("".into()),
        }
    }
}