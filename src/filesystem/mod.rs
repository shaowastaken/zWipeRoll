pub mod recycle_bin;
pub mod scanner;
pub mod volumes;

pub use recycle_bin::{move_to_recycle_bin, DeletionResult};
pub use scanner::{
    discover_global_scan_roots, scan_directory, scan_image_folders, ImageFolder, ScanProgress,
    ScanReport,
};
