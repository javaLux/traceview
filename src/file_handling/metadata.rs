use crate::utils;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::SystemTime};
/// Represents file metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMetadata {
    pub created: Option<SystemTime>,
    pub last_access: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub read_only: bool,
    pub size: u64,
}

impl FileMetadata {
    /*
    Convert file metadata to a table row format
    */
    pub fn get_metadata_rows<P: AsRef<Path>>(&self, file_path: P) -> Vec<Vec<String>> {
        let created = self.created.map_or("Unknown".to_string(), |time| {
            utils::system_time_to_readable(&time)
        });

        let last_access = self.last_access.map_or("Unknown".to_string(), |time| {
            utils::system_time_to_readable(&time)
        });

        let modified = self.modified.map_or("Unknown".to_string(), |time| {
            utils::system_time_to_readable(&time)
        });

        let read_only = if self.read_only {
            "Yes".to_string()
        } else {
            "No".to_string()
        };

        let file_type = file_format::FileFormat::from_file(file_path)
            .ok()
            .map_or("File".to_string(), |file_format| {
                format!("File [{}]", file_format.name())
            });

        vec![
            vec!["Created".to_string(), created],
            vec!["Last used".to_string(), last_access],
            vec!["Modified".to_string(), modified],
            vec!["Read only".to_string(), read_only],
            vec![
                "Size".to_string(),
                utils::convert_bytes_to_human_readable(self.size),
            ],
            vec!["Type".to_string(), file_type.to_string()],
        ]
    }
}

/// Represents directory metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirMetadata {
    pub dir_name: String,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub file_count: usize,
    pub dir_count: usize,
    pub total_size: u64,
}

impl DirMetadata {
    /*
    Convert directory metadata to a table row format
    */
    pub fn get_metadata_rows(&self) -> Vec<Vec<String>> {
        let created = self.created.map_or("Unknown".to_string(), |time| {
            utils::system_time_to_readable(&time)
        });

        let modified = self.modified.map_or("Unknown".to_string(), |time| {
            utils::system_time_to_readable(&time)
        });

        vec![
            vec!["Created".to_string(), created],
            vec!["Modified".to_string(), modified],
            vec![
                "Included Directories".to_string(),
                self.dir_count.to_string(),
            ],
            vec!["Included Files".to_string(), self.file_count.to_string()],
            vec![
                "Total size".to_string(),
                utils::convert_bytes_to_human_readable(self.total_size),
            ],
            vec!["Type".to_string(), "Directory".to_string()],
        ]
    }
}
