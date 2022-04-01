use anyhow::{anyhow, Error, Result};
use api::{Api, Id, MediaItemResponse, MediaItemSearchRequest};
use args::Cli;
use clap::StructOpt;
use client::get_api;
use config::{configure, does_config_exist, Configuration, LocalAlbum};
use directories::ProjectDirs;
use futures::{stream, StreamExt, TryStreamExt};
use item::{download_file, Item, MediaType};
use std::fs::create_dir_all;

mod album;
mod api;
mod args;
mod client;
mod config;
mod item;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_dirs = ProjectDirs::from("app", "Redwarp", "Sync Google Photo")
        .expect("Couldn't create a project dir");

    let should_configure = if cli.configure {
        true
    } else {
        !does_config_exist(&project_dirs)
    };

    if should_configure {
        configure(&project_dirs).await?;
    } else {
        // dostuff().await?;
        synchronize(&project_dirs).await?;
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

async fn get_next_page(api: &Api, album_id: &Id, next_page_token: Option<String>) -> Result<Page> {
    let media_response: MediaItemResponse = api
        .post(
            "https://photoslibrary.googleapis.com/v1/mediaItems:search",
            &MediaItemSearchRequest {
                album_id,
                page_size: Some(50),
                page_token: next_page_token,
            },
        )
        .await?;

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

                Some(Item::new(item.filename, item.base_url, media_type))
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

async fn download_all(api: &Api, local_album: &LocalAlbum) -> Result<()> {
    enum Paging {
        Starting,
        Next(String),
        Finish,
    }

    let stream = stream::try_unfold(Paging::Starting, |token| async {
        match token {
            Paging::Starting => {
                let page = get_next_page(api, &local_album.album_id, None).await?;
                let next = match &page.next_page_token {
                    Some(token) => Paging::Next(token.clone()),
                    None => Paging::Finish,
                };
                Ok::<_, Error>(Some((page, next)))
            }
            Paging::Next(next_page_token) => {
                let page = get_next_page(api, &local_album.album_id, Some(next_page_token)).await?;
                let next = match &page.next_page_token {
                    Some(token) => Paging::Next(token.clone()),
                    None => Paging::Finish,
                };
                Ok(Some((page, next)))
            }
            Paging::Finish => Ok(None),
        }
    });

    let items = stream.flat_map(|page_result: Result<_, _>| match page_result {
        Ok(page) => stream::iter(page.items.into_iter().map(Ok).collect::<Vec<_>>()),
        _ => stream::iter(vec![Err(anyhow!("Error with page"))]),
    });

    items
        .try_for_each_concurrent(4, |item| async move {
            download_file(&item, &local_album.path).await
        })
        .await?;

    Ok(())
}

async fn synchronize(project_dirs: &ProjectDirs) -> Result<()> {
    let configuration = Configuration::load(project_dirs)?;
    let api = get_api().await?;

    for local_album in &configuration.local_albums {
        println!("Synchronizing {}", local_album.name);
        create_dir_all(&local_album.path)?;
        download_all(api, local_album).await?;
    }

    Ok(())
}
