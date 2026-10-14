use super::volumes::{discover_volume_mounts, volume_for_path, VolumeMount};
use std::path::{Path, PathBuf};

pub struct DeletionResult {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub errors: Vec<(PathBuf, String)>,
}

pub fn move_to_recycle_bin(
    paths_and_sizes: &[(PathBuf, u64)],
    expected_volumes: &[VolumeMount],
) -> DeletionResult {
    let current = discover_volume_mounts();
    if !volume_identities_match(paths_and_sizes, expected_volumes, &current) {
        return DeletionResult {
            deleted_count: 0,
            freed_bytes: 0,
            errors: paths_and_sizes.iter().map(|(path, _)| (path.clone(), "Volume unavailable or changed. No files were deleted; reconnect the original volume.".into())).collect(),
        };
    }
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
                if !volume_identities_match(
                    &[(path.clone(), *size)],
                    expected_volumes,
                    &discover_volume_mounts(),
                ) {
                    errors.push((
                        path.clone(),
                        "Volume changed before retry; file retained.".into(),
                    ));
                    continue;
                }
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

fn volume_identities_match(
    paths: &[(PathBuf, u64)],
    expected: &[VolumeMount],
    current: &[VolumeMount],
) -> bool {
    paths
        .iter()
        .all(|(path, _)| match volume_for_path(path, expected) {
            Some(old) => volume_for_path(path, current).is_some_and(|new| new.id == old.id),
            None => true,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recycled_paths_reject_reused_and_disconnected_volumes() {
        let expected = vec![VolumeMount {
            id: "original".into(),
            root: "E:\\".into(),
        }];
        let paths = vec![(PathBuf::from("E:\\photos\\one.jpg"), 10)];
        assert!(!volume_identities_match(&paths, &expected, &[]));
        let mut actual = expected.clone();
        actual[0].id = "replacement".into();
        assert!(!volume_identities_match(&paths, &expected, &actual));
        assert!(volume_identities_match(&paths, &expected, &expected));
    }
}
