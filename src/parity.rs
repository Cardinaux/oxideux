/// This module is for "parity" related actions. That means anything to do with parity root
/// operations is usually handled here, such as listing files in the parity root and getting
/// relevant data. Much like how [`config`] is for config file operations, parity is for the parity
/// root.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub length: u32,
}

pub fn get_file_entry(path: PathBuf) -> Result<Entry> {
    if !path.is_file() {
        return Err(anyhow::anyhow!(format!("Path is not a file: {:?}", path)));
    }

    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let length = fs::metadata(&path)?.len() as u32;

    Ok(Entry {
        name,
        path: path.clone(),
        length,
    })
}

pub fn get_file_entries(path: PathBuf) -> Result<Vec<Entry>> {
    let mut entries = vec![];

    let read_dir = fs::read_dir(path)?;
    for res in read_dir {
        let entry = res?;

        if entry.metadata()?.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let length = entry.metadata()?.len() as u32;

        entries.push(Entry { name, path, length });
    }

    Ok(entries)
}
