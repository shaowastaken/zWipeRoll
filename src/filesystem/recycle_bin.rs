use std::path::{Path, PathBuf};

pub struct DeletionResult {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub errors: Vec<(PathBuf, String)>,
}

pub fn move_to_recycle_bin(paths_and_sizes: &[(PathBuf, u64)]) -> DeletionResult {
    let mut deleted_count = 0;
    let mut freed_bytes = 0;
    let mut errors = Vec::new();

    // Group paths for batch delete
    let paths: Vec<&Path> = paths_and_sizes.iter().map(|(p, _)| p.as_path()).collect();

    match trash::delete_all(&paths) {
        Ok(_) => {
            deleted_count = paths_and_sizes.len();
            freed_bytes = paths_and_sizes.iter().map(|(_, sz)| *sz).sum();
        }
        Err(_e) => {
            // If batch fails, try one by one so partial success is preserved
            for (path, size) in paths_and_sizes {
                match trash::delete(path) {
                    Ok(_) => {
                        deleted_count += 1;
                        freed_bytes += size;
                    }
                    Err(err) => {
                        errors.push((path.clone(), err.to_string()));
                    }
                }
            }
        }
    }

    DeletionResult {
        deleted_count,
        freed_bytes,
        errors,
    }
}
