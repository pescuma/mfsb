use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::workspace::SharedFolder;
use anyhow::{Error, Result};
use itertools::Itertools;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use uuid::Uuid;

#[derive(Clone)]
pub struct FoldersDB {
    pool: Pool<SqliteConnectionManager>,
}

impl FoldersDB {
    pub fn build(path: &Path) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager)?;

        {
            let mut conn = pool.get()?;

            embedded::migrations::runner().run(conn.deref_mut())?;

            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
        }

        Ok(Self { pool })
    }

    pub fn folder_query_or_insert_by_path(&self, path: &Path) -> Result<SharedFolder> {
        if let Some(existing) = self.folder_query_by_path(path)? {
            return Ok(existing);
        }

        let result = SharedFolder::build(path);
        self.folder_insert(&result)?;
        Ok(result)
    }

    pub fn folder_query_by_path(&self, path: &Path) -> Result<Option<SharedFolder>> {
        let mut result = Vec::new();

        let conn = self.pool.get()?;

        let mut stmt = conn.prepare("SELECT * FROM shared_folders WHERE path = ?")?;
        let mut rows = stmt.query([path.to_str()])?;

        while let Some(row) = rows.next()? {
            let id: Uuid = row.get(0)?;
            let path: String = row.get(1)?;

            result.push(SharedFolder::new(id, PathBuf::from(path)));
        }

        Ok(result.into_iter().at_most_one()?)
    }

    pub fn folder_insert(&self, folder: &SharedFolder) -> Result<()> {
        let conn = self.pool.get()?;

        conn.execute(
            "INSERT INTO shared_folders (id, path) VALUES (?, ?)", //
            (&folder.id, folder.path.to_str()),
        )?;

        Ok(())
    }
}

mod embedded {
    use refinery::embed_migrations;

    embed_migrations!("migrations");
}
