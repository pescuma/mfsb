use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Error, Result};
use directories::ProjectDirs;
use itertools::Itertools;
use uuid::Uuid;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

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

        let manager = r2d2_sqlite::SqliteConnectionManager::file(data_dir.join("folders.db"));
        let folders_db_pool = r2d2::Pool::new(manager)?;

        {
            let mut conn = folders_db_pool.get()?;
            embedded::migrations::runner().run(conn.deref_mut())?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
        }

        let data = WorkspaceData {
            config_dir,
            data_dir,
            folders_db_pool,
            folders: HashMap::new(),
        };

        Ok(Workspace {
            data: Arc::new(Mutex::new(data)),
        })
    }

    pub fn get_folder(&mut self, path: &Path) -> Result<Arc<SharedFolder>> {
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
    folders_db_pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    folders: HashMap<PathBuf, Arc<SharedFolder>>,
}

impl WorkspaceData {
    fn get_folder(&mut self, path: PathBuf) -> Result<Arc<SharedFolder>> {
        if let Some(existing) = self.folders.get(&path) {
            Ok(existing.clone())
        } else {
            let result = Arc::new(self.query_or_insert_folder(&path)?);
            self.folders.insert(path, result.clone());
            Ok(result)
        }
    }

    fn query_or_insert_folder(&self, path: &Path) -> Result<SharedFolder> {
        if let Some(existing) = self.query_folder(path)? {
            return Ok(existing);
        }

        let result = SharedFolder::build(path);
        self.insert_folder(&result)?;
        Ok(result)
    }

    fn query_folder(&self, path: &Path) -> Result<Option<SharedFolder>> {
        let mut result = Vec::new();

        let conn = self.folders_db_pool.get()?;

        let mut stmt = conn.prepare("SELECT * FROM shared_folders WHERE path = ?")?;
        let mut rows = stmt.query([path.to_str()])?;

        while let Some(row) = rows.next()? {
            let id: Uuid = row.get(0)?;
            let path: String = row.get(1)?;

            result.push(SharedFolder::new(id, PathBuf::from(path)));
        }

        Ok(result.into_iter().at_most_one()?)
    }

    fn insert_folder(&self, folder: &SharedFolder) -> Result<()> {
        let conn = self.folders_db_pool.get()?;

        conn.execute(
            "INSERT INTO shared_folders (id, path) VALUES (?, ?)", //
            (&folder.id, folder.path.to_str()),
        )?;

        Ok(())
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
