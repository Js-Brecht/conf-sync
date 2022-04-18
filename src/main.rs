// extern crate hyper;
// extern crate hyper_rustls;
extern crate google_drive3 as drive3;
use std::path::Path;
use std::fs::File;
use drive3::{api};
use drive3::{Error};
use drive3::{DriveHub, hyper, hyper_rustls};
use yup_oauth2::{
    InstalledFlowAuthenticator,
    InstalledFlowReturnMethod,
};

pub mod fs;
// pub mod config;
pub mod config_handler;
mod auth;
mod token_storage;

use config_handler::ConfigHandler;
use token_storage::KeychainStorage;

#[tokio::main]
async fn main() {
    let cfg = ConfigHandler::new().await;
    let oauth_config = cfg.oauth_config.clone();
    let app_secret = cfg.get_app_secret();
    let fs_keystorage_path = cfg.base_paths.data_dir.join("keystorage.json");

    let key_storage = match KeychainStorage::new(None).await {
        Ok(storage) => storage,
        Err(err) => {
            panic!("Unable to initialize keychain storage: {}", err);
        }
    };

    let auth_handler = InstalledFlowAuthenticator::builder(
        app_secret,
        InstalledFlowReturnMethod::HTTPRedirect,
    )
        .with_storage(Box::new(key_storage))
        .flow_delegate(Box::new(auth::WebLauncherInstallFlowDelegate))
        .build().await.unwrap();

    let hub = DriveHub::new(
        hyper::Client::builder().build(
            hyper_rustls::HttpsConnector::with_native_roots()
        ),
        auth_handler
    );

    let mut req = hub.files().list();
    
    for scope in oauth_config.scopes.iter() {
        req = req.add_scope(scope);
    }
    
    let result = req.doit().await;
    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
            Error::HttpError(_)
            |Error::Io(_)
            |Error::MissingAPIKey
            |Error::MissingToken(_)
            |Error::Cancelled
            |Error::UploadSizeLimitExceeded(_, _)
            |Error::Failure(_)
            |Error::BadRequest(_)
            |Error::FieldClash(_)
            |Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success: {:?}", res)
    }

    let upload_file = match File::open(
        Path::new("/home/jeremy/.local/share/conf-sync/keystorage.json")
    ) {
        Ok(f) => f,
        Err(err) => {
            panic!("Could not open file: {}", err);
        }
    };

    let file_req = api::File {
        name: Some("keystorage.json".into()),
        ..api::File::default()
    };

    let push = hub.files().create(file_req).upload(
        upload_file,
        "application/json".parse().unwrap(),
    ).await;

    match push {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
            Error::HttpError(_)
            |Error::Io(_)
            |Error::MissingAPIKey
            |Error::MissingToken(_)
            |Error::Cancelled
            |Error::UploadSizeLimitExceeded(_, _)
            |Error::Failure(_)
            |Error::BadRequest(_)
            |Error::FieldClash(_)
            |Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success: {:?}", res)
    }

}
