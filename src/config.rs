use std::path::{Path, PathBuf};

use directories::ProjectDirs;

const CONFIG_FILE: &str = "config.json";

struct Configuration {}

struct Folder {
    path: PathBuf,
    album_id: String,
}

pub async fn configure(project_dirs: &ProjectDirs) {}

pub fn does_config_exist(project_dirs: &ProjectDirs) -> bool {
    project_dirs.config_dir().join(CONFIG_FILE).exists()
}
