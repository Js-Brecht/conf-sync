extern crate yup_oauth2;
use std::ops::{Deref};
use std::future::Future;
use std::pin::Pin;
use yup_oauth2::authenticator_delegate::{
    InstalledFlowDelegate,
};
use async_trait::async_trait;

#[derive(Copy, Clone)]
pub struct WebLauncherInstallFlowDelegate;

impl WebLauncherInstallFlowDelegate {
    async fn wait_for_complete<'a>(
        &'a self,
        need_code: bool
    ) -> Result<String, String> {
        use tokio::io::AsyncBufReadExt;
        if need_code {
            println!(
                "Please follow the instructions in your browser, then enter the \
                 code displayed here: "
            );
    
            let mut user_input = String::new();
            tokio::io::BufReader::new(tokio::io::stdin())
                .read_line(&mut user_input)
                .await
                .map_err(|e| format!("couldn't read code: {}", e))?;
            // remove trailing whitespace.
            user_input.truncate(user_input.trim_end().len());
            Ok(user_input)
        } else {
            println!("Continuing authentication in the browser... ");
            Ok(String::new())
        }
    }
}

#[async_trait]
impl InstalledFlowDelegate for WebLauncherInstallFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        if url.is_empty() {
            panic!("Invalid authentication url provided: {}", url);
        }
        webbrowser::open(url).unwrap();
        Box::pin(self.wait_for_complete(need_code))
    }
}