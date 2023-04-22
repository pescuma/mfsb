use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Error, Result};
use flume::{Receiver, Sender};

use mfsb::snapshot::builder::SnapshotBuilder;
use mfsb::*;

fn main() -> Result<()> {
    let mut ws = workspace::Workspace::build()?;

    let root = PathBuf::from("C:\\Users\\rdomenecci\\Books");

    let folder = ws.get_shared_item(&root)?;

    let snapshot = SnapshotBuilder::new(folder);
    create_snapshot(snapshot);

    Ok(())
}

fn create_snapshot(snapshot: Arc<SnapshotBuilder>) {
    let (pipeline, tx, rx) = pipeline::Pipeline::new(1);

    tx.send(snapshot.clone()).unwrap();
    drop(tx);

    for _ in rx {}

    pipeline.join_threads();

    assert!(snapshot.is_complete());
}
