use std::{fs::read_dir, path::PathBuf};

use anyhow::Result;

pub mod bootstrap;
pub mod decoders;
pub mod get_shreds;
pub mod graph;
pub mod target_dexes;
pub mod transaction_decoders;

pub fn get_all_pool_files(data_folder_path: &str) -> Result<Vec<PathBuf>> {
    Ok(Vec::from_iter(
        read_dir(data_folder_path)?
            .filter_map(anyhow::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("json")),
    ))
}
