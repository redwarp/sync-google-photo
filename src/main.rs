use anyhow::{anyhow, Error, Result};
use api::Album;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use directories::ProjectDirs;
use file_picker::{FilePicker, FileType};
use futures::TryStreamExt;
use futures::{stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use std::fs::{self, create_dir_all, File};
use std::io::{copy, Cursor};
use std::path::PathBuf;

use crate::api::{
    AlbumsListRequest, AlbumsListResponse, MediaItemResponse, MediaItemSearchRequest,
    SharedAlbumsListResponse,
};

mod api;

#[tokio::main]
async fn main() -> Result<()> {
    dostuff().await?;

    Ok(())
}

async fn dostuff() -> Result<()> {
    let project_dirs = ProjectDirs::from("app", "Redwarp", "Sync Google Photo")
        .expect("Couldn't create a project dir");
    let config_dir = project_dirs.config_dir();
    create_dir_all(config_dir)?;

    let secret = yup_oauth2::parse_application_secret(include_bytes!("client_secrets.json"))
        .expect("Should be valid");

    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(config_dir.join("tokencache.json"))
    .build()
    .await;

    match auth {
        Ok(auth) => {
            let scopes = &["https://www.googleapis.com/auth/photoslibrary.readonly"];

            let token = auth.token(scopes).await?;

            let mut headers = HeaderMap::new();
            let mut auth_value: HeaderValue = format!("Bearer {}", token.as_str()).parse()?;
            auth_value.set_sensitive(true);

            headers.insert(AUTHORIZATION, auth_value);

            let client = Client::builder().default_headers(headers).build()?;

            let album_types = &["Private albums", "Shared albums", "Cancel"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select an album")
                .default(0)
                .items(album_types)
                .interact()?;

            let albums = match selection {
                0 => list_albums(&client).await,
                1 => list_shared_albums(&client).await,
                _ => return Ok(()),
            }?;

            let album_names: Vec<_> = albums.iter().map(|album| &album.title).collect();

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select an album")
                .default(0)
                .items(&album_names)
                .interact()?;

            let album_id = &albums[selection].id;

            let item_list = list_items(&client, album_id).await?;

            let choices = &["Download all", "Pick one"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("What next?")
                .default(0)
                .items(choices)
                .interact()?;

            match selection {
                0 => {
                    download_all(&client, album_id).await?;
                }
                _ => {
                    let item_names: Vec<_> = item_list.iter().map(|item| &item.filename).collect();

                    let selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select a media")
                        .default(0)
                        .items(&item_names)
                        .interact()?;

                    download_file(&item_list[selection]).await?;
                }
            }
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

#[derive(Clone)]
enum MediaType {
    Photo,
    Video,
}

async fn list_shared_albums(client: &Client) -> Result<Vec<Album>> {
    let url = "https://photoslibrary.googleapis.com/v1/sharedAlbums";

    let request_body = AlbumsListRequest::default();

    let response = client.get(url).query(&request_body).send().await?;
    println!("Request: {:?}", response);

    let text = response.text().await?;
    println!("Response: {}", text);

    // let album_response: AlbumsListResponse = response.json().await?;
    let album_response: SharedAlbumsListResponse = serde_json::from_str(&text)?;

    if let Some(albums) = album_response.shared_albums {
        Ok(albums
            .into_iter()
            .filter_map(|album| {
                if let Some(title) = album.title {
                    Some(Album {
                        id: album.id,
                        title,
                        product_url: album.product_url,
                    })
                } else {
                    None
                }
            })
            .collect())
    } else {
        Ok(vec![])
    }
}

async fn list_albums(client: &Client) -> Result<Vec<Album>> {
    let url = "https://photoslibrary.googleapis.com/v1/albums";

    let request_body = AlbumsListRequest::default();

    let response = client.get(url).query(&request_body).send().await?;
    println!("Request: {:?}", response);

    let text = response.text().await?;
    println!("Response: {}", text);

    // let album_response: AlbumsListResponse = response.json().await?;
    let album_response: AlbumsListResponse = serde_json::from_str(&text)?;

    if let Some(albums) = album_response.albums {
        Ok(albums
            .into_iter()
            .filter_map(|album| {
                if let Some(title) = album.title {
                    Some(Album {
                        id: album.id,
                        title,
                        product_url: album.product_url,
                    })
                } else {
                    None
                }
            })
            .collect())
    } else {
        Ok(vec![])
    }
}

async fn list_items(client: &Client, album_id: &str) -> Result<Vec<Item>> {
    let url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";

    let request_body = serde_json::to_string(&MediaItemSearchRequest {
        album_id: album_id.to_string(),
        page_size: Some(100),
        page_token: None,
    })?;

    let response = client.post(url).body(request_body).send().await?;

    let media_response: MediaItemResponse = response.json().await?;
    if let Some(media_items) = media_response.media_items {
        Ok(media_items
            .into_iter()
            .filter_map(|item| {
                let media_type = if item.media_metadata.photo.is_some() {
                    MediaType::Photo
                } else if item.media_metadata.video.is_some() {
                    MediaType::Video
                } else {
                    return None;
                };

                Some(Item {
                    filename: item.filename,
                    base_url: item.base_url,
                    media_type,
                })
            })
            .collect())
    } else {
        Ok(vec![])
    }
}

#[derive(Clone)]
struct Item {
    filename: String,
    base_url: String,
    media_type: MediaType,
}

async fn download_file(item: &Item) -> Result<()> {
    println!("Downloading {}", item.filename);
    let folder = "downloads";
    let url = match &item.media_type {
        MediaType::Photo => format!("{}={}", item.base_url, "d"),
        MediaType::Video => format!("{}={}", item.base_url, "dv"),
    };

    fs::create_dir_all(folder)?;

    let mut response = reqwest::get(url).await?;

    let filename = PathBuf::from(folder).join(&item.filename);
    let mut file = File::create(filename)?;

    while let Some(chunk) = response.chunk().await? {
        let mut cursor = Cursor::new(chunk);
        copy(&mut cursor, &mut file)?;
    }

    Ok(())
}

#[derive(Default)]
struct Page {
    items: Vec<Item>,
    next_page_token: Option<String>,
}

impl Extend<Page> for Page {
    fn extend<T: IntoIterator<Item = Page>>(&mut self, iter: T) {
        for page in iter {
            self.items.extend(page.items)
        }
    }
}

async fn get_next_page(
    client: &Client,
    album_id: &str,
    next_page_token: Option<String>,
) -> Result<Page> {
    let url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";

    let request_body = serde_json::to_string(&MediaItemSearchRequest {
        album_id: album_id.to_string(),
        page_size: Some(50),
        page_token: next_page_token,
    })?;

    let response = client.post(url).body(request_body).send().await?;

    let media_response: MediaItemResponse = response.json().await?;
    let items = if let Some(media_items) = media_response.media_items {
        media_items
            .into_iter()
            .filter_map(|item| {
                let media_type = if item.media_metadata.photo.is_some() {
                    MediaType::Photo
                } else if item.media_metadata.video.is_some() {
                    MediaType::Video
                } else {
                    return None;
                };

                Some(Item {
                    filename: item.filename,
                    base_url: item.base_url,
                    media_type,
                })
            })
            .collect()
    } else {
        vec![]
    };

    Ok(Page {
        items,
        next_page_token: media_response.next_page_token,
    })
}

async fn download_all(client: &Client, album_id: &str) -> Result<()> {
    enum Paging {
        Starting,
        Next(String),
        Finish,
    }

    let stream = stream::try_unfold(Paging::Starting, |token| async {
        match token {
            Paging::Starting => {
                let page = get_next_page(client, album_id, None).await?;
                let next = match &page.next_page_token {
                    Some(token) => Paging::Next(token.clone()),
                    None => Paging::Finish,
                };
                Ok::<_, Error>(Some((page, next)))
            }
            Paging::Next(next_page_token) => {
                let page = get_next_page(client, album_id, Some(next_page_token)).await?;
                let next = match &page.next_page_token {
                    Some(token) => Paging::Next(token.clone()),
                    None => Paging::Finish,
                };
                Ok(Some((page, next)))
            }
            Paging::Finish => Ok(None),
        }
    });
    // pin_mut!(stream);

    let items = stream.flat_map(|page_result: Result<_, _>| match page_result {
        Ok(page) => stream::iter(page.items.into_iter().map(Ok).collect::<Vec<_>>()),
        _ => stream::iter(vec![Err(anyhow!("Error with page"))]),
    });

    items
        .try_for_each_concurrent(4, |item| async move { download_file(&item).await })
        .await?;

    Ok(())
}
