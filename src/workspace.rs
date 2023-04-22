use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Error, Result};
use directories::ProjectDirs;
use uuid::Uuid;

use crate::db::workspace_db::WorkspaceDB;

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

        let workspace_db = WorkspaceDB::build(&data_dir.join("workspace.db"))?;

        let data = WorkspaceData {
            config_dir,
            data_dir,
            workspace_db,
        };

        Ok(Workspace {
            data: Arc::new(Mutex::new(data)),
        })
    }

    pub fn get_shared_item(&mut self, path: &Path) -> Result<SharedItem> {
        let path = path.canonicalize()?;

        self.lock_data().get_shared_item(path)
    }

    fn lock_data(&mut self) -> MutexGuard<'_, WorkspaceData> {
        self.data.lock().unwrap()
    }
}

struct WorkspaceData {
    config_dir: PathBuf,
    data_dir: PathBuf,
    workspace_db: WorkspaceDB,
}

impl WorkspaceData {
    fn get_shared_item(&mut self, path: PathBuf) -> Result<SharedItem> {
        self.workspace_db
            .shared_items
            .query_or_insert_by_path(&path)
    }
}

#[derive(Debug)]
pub struct SharedItem {
    pub id: Uuid,
    pub path: PathBuf,
}

impl SharedItem {
    pub fn new(id: Uuid, path: PathBuf) -> Self {
        Self { id, path }
    }

    pub fn build(path: &Path) -> Self {
        Self::new(Uuid::new_v4(), path.to_owned())
    }
}
