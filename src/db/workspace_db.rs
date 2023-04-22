use std::path::{Path, PathBuf};

use crate::db::open_db;
use crate::workspace::SharedItem;
use anyhow::Result;
use itertools::Itertools;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use uuid::Uuid;

#[derive(Clone)]
pub struct WorkspaceDB {
    pub shared_items: SharedItemsDB,
}

impl WorkspaceDB {
    pub fn build(path: &Path) -> Result<Self> {
        let pool = open_db(path, embedded::migrations::runner())?;

        Ok(Self {
            shared_items: SharedItemsDB { pool },
        })
    }
}

#[derive(Clone)]
pub struct SharedItemsDB {
    pool: Pool<SqliteConnectionManager>,
}

impl SharedItemsDB {
    pub fn query_or_insert_by_path(&self, path: &Path) -> Result<SharedItem> {
        if let Some(existing) = self.query_by_path(path)? {
            return Ok(existing);
        }

        let result = SharedItem::build(path);
        self.insert(&result)?;
        Ok(result)
    }

    pub fn query_by_path(&self, path: &Path) -> Result<Option<SharedItem>> {
        let mut result = Vec::new();

        let conn = self.pool.get()?;

        let mut stmt = conn.prepare("SELECT * FROM shared_folders WHERE path = ?")?;
        let mut rows = stmt.query([path.to_str()])?;

        while let Some(row) = rows.next()? {
            let id: Uuid = row.get(0)?;
            let path: String = row.get(1)?;

            result.push(SharedItem::new(id, PathBuf::from(path)));
        }

        Ok(result.into_iter().at_most_one()?)
    }

    pub fn insert(&self, folder: &SharedItem) -> Result<()> {
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

    embed_migrations!("migrations/workspace_db");
}
