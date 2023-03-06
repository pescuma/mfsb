use anyhow::{Error, Result};
use relative_path::RelativePathBuf;
use std::path::PathBuf;
use std::{fs, io};

pub fn path_walk(
    root: PathBuf,
    mut cb: impl FnMut(PathBuf, RelativePathBuf, io::Result<fs::Metadata>),
) -> Result<()> {
    let root_metadata = fs::metadata(&root)?;

    if root_metadata.is_file() {
        let relative = RelativePathBuf::from_path(".").unwrap().normalize();
        cb(root, relative, Ok(root_metadata));

        Ok(())
    } else if root_metadata.is_dir() {
        let mut queue = vec![(root.clone(), Some(root_metadata))];

        while let Some((path, path_metadata)) = queue.pop() {
            walk_one(&mut queue, path, path_metadata, &mut |path, result| {
                let relative = RelativePathBuf::from_path(path.strip_prefix(&root).unwrap())
                    .unwrap()
                    .normalize();

                cb(path, relative, result);
            });
        }

        Ok(())
    } else {
        Err(Error::msg(format!(
            "should be a dir or a file (is {:?})",
            root_metadata.file_type()
        )))
    }
}

fn walk_one(
    queue: &mut Vec<(PathBuf, Option<fs::Metadata>)>,
    path: PathBuf,
    path_metadata: Option<fs::Metadata>,
    cb: &mut impl FnMut(PathBuf, io::Result<fs::Metadata>),
) {
    macro_rules! unwrap {
        ($f:expr) => {
            match $f {
                Err(err) => {
                    cb(path, Err(err));
                    return;
                }
                Ok(e) => e,
            }
        };
    }

    let mut found = Vec::new();

    for entry in unwrap!(fs::read_dir(&path)) {
        let entry = unwrap!(entry);
        let entry_path = entry.path();

        if entry_path.starts_with("c:\\Windows") {
            continue;
        }

        match entry.file_type() {
            Err(err) => {
                found.push((entry_path, Some(err)));
            }
            Ok(file_type) if file_type.is_dir() => {
                queue.push((entry_path, None));
            }
            _ => {
                found.push((entry_path, None));
            }
        };
    }

    if let Some(path_metadata) = path_metadata {
        cb(path, Ok(path_metadata));
    } else {
        let path_metadata = unwrap!(fs::metadata(&path));
        cb(path, Ok(path_metadata));
    }

    for (entry_path, entry_err) in found {
        if let Some(err) = entry_err {
            cb(entry_path, Err(err));
            continue;
        }

        match fs::metadata(&entry_path) {
            Err(err) => {
                cb(entry_path, Err(err));
            }
            Ok(entry_metadata) if entry_metadata.is_file() => {
                cb(entry_path, Ok(entry_metadata));
            }
            Ok(entry_metadata) if entry_metadata.is_dir() => {
                // It was a symlink
                queue.push((entry_path, Some(entry_metadata)));
            }
            entry_metadata => {
                panic!(
                    "{:?}: should be dir or file: {:?}",
                    entry_path, entry_metadata
                );
            }
        };
    }
}
