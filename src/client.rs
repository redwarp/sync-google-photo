use anyhow::{anyhow, Result};
use async_once::AsyncOnce;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};

use crate::api::Api;

lazy_static! {
    static ref CLIENT: AsyncOnce<Result<Api>> = AsyncOnce::new(async { init_api().await });
}

pub async fn get_api<'a>() -> Result<&'a Api> {
    let client = CLIENT
        .get()
        .await
        .as_ref()
        .map_err(|_| anyhow!("Error getting the client"));

    client
}

async fn init_api() -> Result<Api> {
    let project_dirs = ProjectDirs::from("app", "Redwarp", "Sync Google Photo")
        .expect("Couldn't create a project dir");
    let config_dir = project_dirs.config_dir();
    std::fs::create_dir_all(config_dir)?;

    let secret = yup_oauth2::parse_application_secret(include_bytes!("client_secrets.json"))
        .expect("Should be valid");

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(config_dir.join("tokencache.json"))
    .build()
    .await?;

    let scopes = &["https://www.googleapis.com/auth/photoslibrary.readonly"];

    let token = auth.token(scopes).await?;

    let mut headers = HeaderMap::new();
    let mut auth_value: HeaderValue = format!("Bearer {}", token.as_str()).parse()?;
    auth_value.set_sensitive(true);

    headers.insert(AUTHORIZATION, auth_value);

    let client = Client::builder().default_headers(headers).build()?;
    let api = Api::new(client);

    Ok(api)
}
