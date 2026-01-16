#![feature(integer_atomics)]
extern crate core;

use std::{fs::read_dir, path::PathBuf};

use anyhow::Result;

mod benchmark_tools;
pub mod bootstrap;
pub mod bootstrap_decoders;
pub mod engine;
pub mod get_shreds;
pub mod graph;
pub mod shred_decoders;
pub mod target_dexes;

pub fn get_all_pool_files(data_folder_path: &str) -> Result<Vec<PathBuf>> {
    Ok(Vec::from_iter(
        read_dir(data_folder_path)?
            .filter_map(anyhow::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("json")),
    ))
}
