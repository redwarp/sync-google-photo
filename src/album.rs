use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::api::{Album, AlbumsListRequest, AlbumsListResponse, Api, SharedAlbumsListResponse};

pub async fn pick_album(api: &Api) -> Result<Album> {
    let album_types = &["Private albums", "Shared albums", "Cancel"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an album")
        .default(0)
        .items(album_types)
        .interact()?;

    let mut albums = match selection {
        0 => list_albums(api).await,
        1 => list_shared_albums(api).await,
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

async fn list_shared_albums(api: &Api) -> Result<Vec<Album>> {
    let album_response: SharedAlbumsListResponse = api
        .get(
            "https://photoslibrary.googleapis.com/v1/sharedAlbums",
            &AlbumsListRequest::default(),
        )
        .await?;

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

async fn list_albums(api: &Api) -> Result<Vec<Album>> {
    let album_response: AlbumsListResponse = api
        .get(
            "https://photoslibrary.googleapis.com/v1/albums",
            &AlbumsListRequest::default(),
        )
        .await?;

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
