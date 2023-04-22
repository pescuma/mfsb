pub mod workspace_db;

use anyhow::{Error, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::ops::DerefMut;
use std::path::Path;

fn open_db(path: &Path, migrations: refinery::Runner) -> Result<Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(path);
    let pool = Pool::new(manager)?;

    {
        let mut conn = pool.get()?;

        migrations.run(conn.deref_mut())?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
    }

    Ok(pool)
}
