use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use reqwest::Client;

use crate::api::{Album, AlbumsListRequest, AlbumsListResponse, SharedAlbumsListResponse};

pub async fn pick_album(client: &Client) -> Result<Album> {
    let album_types = &["Private albums", "Shared albums", "Cancel"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an album")
        .default(0)
        .items(album_types)
        .interact()?;

    let mut albums = match selection {
        0 => list_albums(client).await,
        1 => list_shared_albums(client).await,
        _ => unreachable!("Only two choices"),
    }?;

    let album_names: Vec<_> = albums.iter().map(|album| &album.title).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an album")
        .default(0)
        .items(&album_names)
        .interact()?;

    let album = albums.swap_remove(selection);
    Ok(album)
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
