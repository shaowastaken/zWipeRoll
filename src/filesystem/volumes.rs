use std::path::{Path, PathBuf};

/// A currently mounted path and its Windows volume GUID. Multiple mount paths
/// may refer to the same volume; letters are deliberately not identities.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VolumeMount {
    pub id: String,
    pub root: PathBuf,
}

/// Finds the most specific mount, without requiring the target to exist.
/// This also works for offline catalog paths and respects component boundaries.
pub fn volume_for_path<'a>(path: &Path, mounts: &'a [VolumeMount]) -> Option<&'a VolumeMount> {
    let path = normalized(path);
    mounts
        .iter()
        .filter(|mount| {
            let root = normalized(&mount.root);
            path == root
                || path
                    .strip_prefix(&root)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|mount| normalized(&mount.root).len())
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

/// Enumerates volume GUIDs and every drive-letter/folder mount using Kernel32.
/// Network shares have no Windows volume GUID and are not guessed from letters.
/// See Microsoft FindFirstVolumeW / GetVolumePathNamesForVolumeNameW contracts.
#[cfg(windows)]
pub fn discover_volume_mounts() -> Vec<VolumeMount> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FindFirstVolumeW(name: *mut u16, length: u32) -> *mut std::ffi::c_void;
        fn FindNextVolumeW(handle: *mut std::ffi::c_void, name: *mut u16, length: u32) -> i32;
        fn FindVolumeClose(handle: *mut std::ffi::c_void) -> i32;
        fn GetVolumePathNamesForVolumeNameW(
            name: *const u16,
            paths: *mut u16,
            length: u32,
            required: *mut u32,
        ) -> i32;
    }

    struct VolumeSearch(*mut std::ffi::c_void);
    impl Drop for VolumeSearch {
        fn drop(&mut self) {
            // SAFETY: this handle came from a successful FindFirstVolumeW.
            unsafe { FindVolumeClose(self.0) };
        }
    }

    let mut mounts = Vec::new();
    // GUID paths have a fixed format well below this capacity.
    let mut name = vec![0u16; 1024];
    // SAFETY: name points to writable storage of the supplied size.
    let handle = unsafe { FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32) };
    if handle == -1isize as *mut std::ffi::c_void {
        return mounts;
    }
    let search = VolumeSearch(handle);
    loop {
        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        let id = String::from_utf16_lossy(&name[..end]).to_lowercase();
        let mut paths = vec![0u16; 1024];
        let mut required = 0;
        // SAFETY: name is NUL-terminated by the enumeration API; both outputs
        // are writable and paths has the advertised capacity.
        let mut ok = unsafe {
            GetVolumePathNamesForVolumeNameW(
                name.as_ptr(),
                paths.as_mut_ptr(),
                paths.len() as u32,
                &mut required,
            )
        };
        if ok == 0 && required > paths.len() as u32 {
            paths.resize(required as usize, 0);
            // SAFETY: same valid inputs, now with the requested buffer size.
            ok = unsafe {
                GetVolumePathNamesForVolumeNameW(
                    name.as_ptr(),
                    paths.as_mut_ptr(),
                    paths.len() as u32,
                    &mut required,
                )
            };
        }
        if ok != 0 {
            for path in paths.split(|&c| c == 0).take_while(|part| !part.is_empty()) {
                mounts.push(VolumeMount {
                    id: id.clone(),
                    root: PathBuf::from(OsString::from_wide(path)),
                });
            }
        }
        // SAFETY: live search handle and writable name buffer.
        if unsafe { FindNextVolumeW(search.0, name.as_mut_ptr(), name.len() as u32) } == 0 {
            break;
        }
    }
    mounts.sort_by(|a, b| a.root.cmp(&b.root));
    mounts
}

#[cfg(not(windows))]
pub fn discover_volume_mounts() -> Vec<VolumeMount> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    #[ignore = "host-dependent read-only Win32 probe"]
    fn mounted_volume_discovery_probe() {
        let mounts = discover_volume_mounts();
        eprintln!("Detected mounts: {mounts:?}");
        let current = std::env::current_dir().unwrap();
        let mount = volume_for_path(&current, &mounts).expect("workspace has a volume GUID");
        assert!(mount.id.starts_with("\\\\?\\volume{"));
        assert!(mount.id.ends_with("}\\"));
    }

    #[test]
    fn longest_mount_matches_case_and_component_boundaries() {
        let mounts = vec![
            VolumeMount {
                id: "outer".into(),
                root: PathBuf::from("C:\\"),
            },
            VolumeMount {
                id: "inner".into(),
                root: PathBuf::from("C:\\mounted\\"),
            },
        ];
        assert_eq!(
            volume_for_path(Path::new("c:/MOUNTED/photos/a.jpg"), &mounts)
                .unwrap()
                .id,
            "inner"
        );
        assert_eq!(
            volume_for_path(Path::new("C:\\mounted"), &mounts)
                .unwrap()
                .id,
            "inner"
        );
        assert_eq!(
            volume_for_path(Path::new("C:\\mounted-other\\a.jpg"), &mounts)
                .unwrap()
                .id,
            "outer"
        );
        assert!(volume_for_path(Path::new("D:\\photos"), &mounts).is_none());
    }
}
