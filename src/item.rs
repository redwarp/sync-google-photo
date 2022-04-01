use anyhow::Result;
use exif::{In, Tag};
use reqwest::Client;
use std::{
    fs::{self, File},
    io::{copy, BufReader, Cursor},
    path::{Path, PathBuf},
};
use uuid::Uuid;

use crate::api::{Id, MediaItemResponse, MediaItemSearchRequest};

#[derive(Clone)]
pub enum MediaType {
    Photo,
    Video,
}

#[derive(Clone)]
pub struct Item {
    filename: String,
    base_url: String,
    media_type: MediaType,
}

impl Item {
    pub fn new(filename: String, base_url: String, media_type: MediaType) -> Self {
        Self {
            filename,
            base_url,
            media_type,
        }
    }
}

async fn _list_items(client: &Client, album_id: &Id) -> Result<Vec<Item>> {
    let url = "https://photoslibrary.googleapis.com/v1/mediaItems:search";

    let request_body = serde_json::to_string(&MediaItemSearchRequest {
        album_id,
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

                Some(Item::new(item.filename, item.base_url, media_type))
            })
            .collect())
    } else {
        Ok(vec![])
    }
}

pub async fn download_file<P>(item: &Item, output_folder: P) -> Result<()>
where
    P: AsRef<Path>,
{
    println!("Downloading {}", item.filename);
    let url = match &item.media_type {
        MediaType::Photo => format!("{}={}", item.base_url, "d"),
        MediaType::Video => format!("{}={}", item.base_url, "dv"),
    };

    fs::create_dir_all(&output_folder)?;

    let mut response = reqwest::get(url).await?;

    let temp_filename = Uuid::new_v4();
    let temp_filename = output_folder.as_ref().join(format!("{temp_filename}"));
    let mut file = File::create(&temp_filename)?;

    while let Some(chunk) = response.chunk().await? {
        let mut cursor = Cursor::new(chunk);
        copy(&mut cursor, &mut file)?;
    }

    let filename = best_file_name(&temp_filename, item, &output_folder)?;
    std::fs::rename(temp_filename, &filename)?;

    Ok(())
}

fn best_file_name<P1, P2>(file_path: P1, item: &Item, output_folder: P2) -> Result<PathBuf>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let file_name = match item.media_type {
        MediaType::Photo => match PathBuf::from(&item.filename)
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
        {
            Some(ext) => match ext.as_str() {
                "jpg" | "jpeg" | "png" => {
                    let ext = if ext.as_str() == "jpeg" {
                        "jpg"
                    } else {
                        ext.as_str()
                    };

                    let file = File::open(&file_path)?;
                    let mut bufreader = BufReader::new(&file);
                    let exif_reader = exif::Reader::new();
                    let exif = exif_reader.read_from_container(&mut bufreader)?;
                    if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
                        let sanitize_date = field
                            .display_value()
                            .to_string()
                            .replace(':', "-")
                            .replace(' ', "_");
                        let name = format!("{}.{}", sanitize_date, ext);
                        output_folder.as_ref().join(&name)
                    } else {
                        output_folder.as_ref().join(&item.filename)
                    }
                }
                _ => output_folder.as_ref().join(&item.filename),
            },
            None => output_folder.as_ref().join(&item.filename),
        },
        MediaType::Video => output_folder.as_ref().join(&item.filename),
    };

    Ok(file_name)
}
