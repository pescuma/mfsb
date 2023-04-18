use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Error, Result};
use directories::ProjectDirs;
use uuid::Uuid;

use crate::db::folders_db::FoldersDB;

#[derive(Clone)]
pub struct Workspace {
    data: Arc<Mutex<WorkspaceData>>,
}

impl Workspace {
    pub fn build() -> Result<Workspace> {
        let dirs = ProjectDirs::from("org", "pescuma", "mfsb")
            .ok_or_else(|| Error::msg("Could not find project directories"))?;

        let config_dir = dirs.config_local_dir().to_owned();
        std::fs::create_dir_all(&config_dir)?;

        let data_dir = dirs.data_local_dir().to_owned();
        std::fs::create_dir_all(&data_dir)?;

        let folders_db = FoldersDB::build(&data_dir.join("folders.db"))?;

        let data = WorkspaceData {
            config_dir,
            data_dir,
            folders_db,
        };

        Ok(Workspace {
            data: Arc::new(Mutex::new(data)),
        })
    }

    pub fn get_folder(&mut self, path: &Path) -> Result<SharedFolder> {
        let path = path.canonicalize()?;

        self.lock_data().get_folder(path)
    }

    fn lock_data(&mut self) -> MutexGuard<'_, WorkspaceData> {
        self.data.lock().unwrap()
    }
}

struct WorkspaceData {
    config_dir: PathBuf,
    data_dir: PathBuf,
    folders_db: FoldersDB,
}

impl WorkspaceData {
    fn get_folder(&mut self, path: PathBuf) -> Result<SharedFolder> {
        self.folders_db.folder_query_or_insert_by_path(&path)
    }
}

#[derive(Debug)]
pub struct SharedFolder {
    pub id: Uuid,
    pub path: PathBuf,
}

impl SharedFolder {
    pub fn new(id: Uuid, path: PathBuf) -> Self {
        Self { id, path }
    }

    pub fn build(path: &Path) -> Self {
        Self::new(Uuid::new_v4(), path.to_owned())
    }
}
