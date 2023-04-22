use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::{atomic, Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Error;
use anyhow::Result;
use relative_path::{RelativePath, RelativePathBuf};

use crate::pack::location::PackLocation;
use crate::workspace::SharedItem;

pub struct SnapshotBuilder {
    root: Arc<SharedItem>,
    paths: Mutex<Vec<Arc<PathBuilder>>>,
    paths_count: atomic::AtomicI32,
    error: Mutex<Option<Error>>,
    start: Instant,
}

impl SnapshotBuilder {
    pub fn new(root: SharedItem) -> Arc<SnapshotBuilder> {
        Arc::new(SnapshotBuilder {
            root: Arc::new(root),
            paths: Mutex::new(Vec::new()),
            paths_count: atomic::AtomicI32::new(-1),
            error: Mutex::new(None),
            start: Instant::now(),
        })
    }

    pub fn get_root(&self) -> &Path {
        &self.root.path
    }

    pub fn add_path(
        &self,
        path: PathBuf,
        relative_path: RelativePathBuf,
        metadata: Option<Metadata>,
    ) -> Arc<PathBuilder> {
        let result = PathBuilder::new(path, relative_path, metadata);

        self.paths.lock().unwrap().push(result.clone());

        result
    }

    pub fn set_finished_adding_paths(&self, path_count: u32) {
        assert_eq!(path_count, self.paths.lock().unwrap().len() as u32);

        self.paths_count
            .store(path_count as i32, atomic::Ordering::SeqCst);
    }

    pub fn set_error(&self, err: Error) {
        *self.error.lock().unwrap() = Some(err);
    }

    pub fn get_elapsed_time(&self) -> Duration {
        Instant::now() - self.start
    }

    pub fn is_complete(&self) -> bool {
        return self.paths_count.load(atomic::Ordering::SeqCst) >= 0
            && self.paths.lock().unwrap().iter().all(|p| p.is_complete());
    }
}

pub struct PathBuilder {
    path: PathBuf,
    relative_path: RelativePathBuf,
    metadata: Option<Metadata>,
    chunks: Mutex<Vec<Arc<ChunkBuilder>>>,
    chunk_count: atomic::AtomicI32,
    error: Mutex<Option<Error>>,
    start: Instant,
}

impl PathBuilder {
    fn new(path: PathBuf, relative_path: RelativePathBuf, metadata: Option<Metadata>) -> Arc<PathBuilder> {
        Arc::new(PathBuilder {
            path,
            relative_path,
            metadata,
            chunks: Mutex::new(Vec::new()),
            chunk_count: atomic::AtomicI32::new(-1),
            error: Mutex::new(None),
            start: Instant::now(),
        })
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_relative_path(&self) -> &RelativePath {
        &self.relative_path
    }

    pub fn get_metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn add_chunk(&self, size: u32) -> Arc<ChunkBuilder> {
        let mut chunks = self.chunks.lock().unwrap();

        let result = ChunkBuilder::new(chunks.len() as u32, size);

        chunks.push(result.clone());

        return result;
    }

    pub fn set_finished_adding_chunks(&self, chunk_count: u32) {
        assert_eq!(chunk_count, self.chunks.lock().unwrap().len() as u32);

        self.chunk_count
            .store(chunk_count as i32, atomic::Ordering::SeqCst);
    }

    pub fn set_error(&self, err: Error) {
        *self.error.lock().unwrap() = Some(err);
    }

    pub fn get_elapsed_time(&self) -> Duration {
        Instant::now() - self.start
    }

    pub fn is_complete(&self) -> bool {
        return self.error.lock().unwrap().is_some()
            || (self.chunk_count.load(atomic::Ordering::SeqCst) >= 0
                && self.chunks.lock().unwrap().iter().all(|c| c.is_complete()));
    }
}

pub struct ChunkBuilder {
    index: u32,
    size: u32,
    hash: Mutex<Vec<u8>>,
    pack_location: Mutex<Option<PackLocation>>,
    start: Instant,
}

impl ChunkBuilder {
    fn new(index: u32, size: u32) -> Arc<ChunkBuilder> {
        Arc::new(ChunkBuilder {
            index,
            size,
            hash: Mutex::new(Vec::new()),
            pack_location: Mutex::new(None),
            start: Instant::now(),
        })
    }

    pub fn get_index(&self) -> u32 {
        self.index
    }

    pub fn get_size(&self) -> u32 {
        self.size
    }

    pub fn set_hash(&self, hash: Vec<u8>) {
        *self.hash.lock().unwrap() = hash;
    }

    pub fn set_stored(&self, pack_location: PackLocation) {
        *self.pack_location.lock().unwrap() = Some(pack_location);
    }

    pub fn get_elapsed_time(&self) -> Duration {
        Instant::now() - self.start
    }

    pub fn is_complete(&self) -> bool {
        return !self.hash.lock().unwrap().is_empty() && self.pack_location.lock().unwrap().is_some();
    }
}
