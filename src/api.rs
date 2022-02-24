use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub product_url: String,
}

impl Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiAlbum {
    pub id: String,
    pub title: Option<String>,
    pub product_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumsListResponse {
    pub albums: Option<Vec<ApiAlbum>>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedAlbumsListResponse {
    pub shared_albums: Option<Vec<ApiAlbum>>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumsListRequest {
    page_size: Option<u32>,
    page_token: Option<String>,
    exclude_non_app_created_data: bool,
}

impl Default for AlbumsListRequest {
    fn default() -> Self {
        Self {
            page_size: Some(20),
            page_token: None,
            exclude_non_app_created_data: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItemSearchRequest {
    pub album_id: String,
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub filename: String,
    pub base_url: String,
    pub media_metadata: MediaMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub photo: Option<Photo>,
    pub video: Option<Video>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Photo {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItemResponse {
    pub media_items: Option<Vec<MediaItem>>,
    pub next_page_token: Option<String>,
}
