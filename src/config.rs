use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_file, File},
    path::PathBuf,
    str::FromStr,
};

use crate::{album::pick_album, api::Id, client::get_api};

const CONFIG_FILE: &str = "config.json";
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Serialize, Deserialize)]
pub struct LocalAlbum {
    pub path: PathBuf,
    pub album_id: Id,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub local_albums: Vec<LocalAlbum>,
}

impl Configuration {
    fn save(&self, project_dirs: &ProjectDirs) -> Result<()> {
        create_dir_all(project_dirs.config_dir())?;

        let config_file = project_dirs.config_dir().join(CONFIG_FILE);
        if config_file.exists() {
            remove_file(&config_file)?;
        }
        serde_json::to_writer(&File::create(config_file)?, self)?;

        Ok(())
    }

    pub fn load(project_dirs: &ProjectDirs) -> Result<Self> {
        let config_file = project_dirs.config_dir().join(CONFIG_FILE);
        if config_file.exists() {
            let configuration: Configuration = serde_json::from_reader(&File::open(&config_file)?)?;

            Ok(configuration)
        } else {
            Ok(Configuration {
                local_albums: vec![],
            })
        }
    }

    fn list_albums(&self) {
        if self.local_albums.is_empty() {
            println!("No album yet");
        }

        for local_album in &self.local_albums {
            println!("{}", local_album.name);
        }
    }
}

pub async fn configure(project_dirs: &ProjectDirs) -> Result<()> {
    let choices = vec!["List synchronized albums", "Synchronize new album"];
    let mut configuration = Configuration::load(project_dirs)?;

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&choices)
        .default(0)
        .interact()?;
    match selection {
        0 => configuration.list_albums(),
        1 => {
            add_new_album(&mut configuration, project_dirs).await?;
        }
        _ => unreachable!("Only two choices in the menu"),
    };

    Ok(())
}

pub fn does_config_exist(project_dirs: &ProjectDirs) -> bool {
    project_dirs.config_dir().join(CONFIG_FILE).exists()
}

async fn add_new_album(
    configuration: &mut Configuration,
    project_dirs: &ProjectDirs,
) -> Result<()> {
    let album = pick_album(get_api().await?).await?;
    let path = PathBuf::from_str(MANIFEST_DIR)?
        .join("downloads")
        .join(&album.title.trim());

    configuration.local_albums.push(LocalAlbum {
        path,
        album_id: album.id,
        name: album.title.trim().to_string(),
    });

    configuration.save(project_dirs)?;

    Ok(())
}
