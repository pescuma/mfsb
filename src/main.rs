use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Error;
use anyhow::Result;
use flume::{Receiver, Sender};

use mfsb::ecc::ECC;
use mfsb::pack::builder::PackBuilder;
use mfsb::path_walk::path_walk;
use mfsb::snapshot::builder::SnapshotBuilder;
use mfsb::*;
use mfsb::{compress, encrypt, hash};

fn main() {
    let root = "C:\\Users\\rdomenecci\\Books";
    let snapshot = SnapshotBuilder::new(PathBuf::from(root));
    create_snapshot(snapshot);
}

fn create_snapshot(snapshot: Arc<SnapshotBuilder>) {
    let (pipeline, tx, rx) = pipeline::Pipeline::new(1);

    tx.send(snapshot.clone()).unwrap();
    drop(tx);

    for _ in rx {}

    pipeline.join_threads();

    assert_eq!(snapshot.is_complete(), true);
}
