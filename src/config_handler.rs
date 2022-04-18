use std::io::Write;
use std::path::{Path, PathBuf};
use serde_derive::{ Deserialize, Serialize };
use yup_oauth2::{ ApplicationSecret };
use config::{Config, File, ConfigError};
use directories::BaseDirs;
use crate::fs::{ensure_dir, open_writeable_file, file_exists};

const APP_NAME: &str = "conf-sync";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(unused)]
pub struct OauthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
}

impl ::std::default::Default for OauthConfig {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(unused)]
pub struct DriveConfig {
    app_folder: String,
}

impl ::std::default::Default for DriveConfig {
    fn default() -> Self {
        Self {
            app_folder: APP_NAME.into()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[derive(Default)]
pub struct Settings {
    gdrive: DriveConfig,
    oauth: OauthConfig,
}

#[derive(Clone)]
pub struct BasePaths {
    pub conf_dir: PathBuf,
    pub conf_path: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Clone)]
pub struct ConfigHandler {
    pub app_name: String,
    pub settings: Settings,
    pub oauth_config: OauthConfig,
    pub base_paths: BasePaths,
}

impl ConfigHandler {
    pub async fn new() -> Self {

        let conf_path = settings_path().await;

        let result = Config::builder()
            .add_source(File::from(conf_path.clone()).required(true))
            .build();

        let deserialized: Result<Settings, ConfigError> = match result {
            Ok(content) => {
                println!("{:#?}", content);
                content.try_deserialize()
            },
            Err(e) => {
                if !file_exists(&conf_path).await {
                    Ok(init_default_config(&conf_path).await)
                } else {
                    panic!("Unable to read settings: {}", e);
                }
            }
        };

        match deserialized {
            Ok(content) => {
                ConfigHandler {
                    app_name: APP_NAME.into(),
                    settings: content.clone(),
                    oauth_config: content.oauth.clone(),
                    base_paths: BasePaths {
                        conf_path: conf_path.clone(),
                        conf_dir: PathBuf::from(conf_path.clone().parent().unwrap()),
                        data_dir: data_path().await,
                    }
                }
            }
            Err(error) => {
                panic!("Unable to parse settings: {}", error);
            }
        }
    }

    pub fn get_app_secret(&self) -> ApplicationSecret {
        let cfg = self.oauth_config.clone();
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

async fn init_default_config(conf_path: &Path) -> Settings {
    let default_config = Settings::default();
    let conf_file = open_writeable_file(conf_path).await.unwrap();
    let result = std::fs::File::from(conf_file.try_into_std().unwrap()).write(
        toml::to_string(&default_config).unwrap().as_bytes()
    );
    match result {
        Ok(_) => {
            default_config
        },
        Err(error) => {
            panic!(
                "Error occurred while initializing default configuration: {}",
                error,
            );
        }
    }
}

async fn data_path() -> PathBuf {
    let base_dirs = BaseDirs::new();
    let dir_path = base_dirs.data_local_dir().join(APP_NAME);
    match ensure_dir(dir_path.as_ref()).await {
        Ok(success) => success,
        Err(err) => {
            panic!("Error while ensuring configuration directory: {}", err);
        }
    }
    dir_path.clone()
}

async fn settings_path() -> PathBuf {
    let base_dirs = BaseDirs::new();
    let dir_path = base_dirs.config_dir().join(APP_NAME);
    let conf_path = dir_path.join("settings.toml");
    match ensure_dir(dir_path.as_ref()).await {
        Ok(success) => success,
        Err(err) => {
            panic!("Error while ensuring configuration directory: {}", err);
        }
    }
    conf_path.clone()
}