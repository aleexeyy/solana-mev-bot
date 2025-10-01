use std::collections::HashSet;
use reqwest::Url;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use crate::bootstrap::pool_schema::TokenInfo;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Deserializer;

#[derive(Deserialize)]
struct MeteoraPool {
    pool_address: Option<String>,
    token_a_mint: Option<String>,
    token_b_mint: Option<String>,
    token_a_vault: Option<String>,
    token_b_vault: Option<String>,
    token_a_symbol: Option<String>,
    token_b_symbol: Option<String>,
    pool_type: Option<String>,
    base_fee: Option<u32>,
    dynamic_fee: Option<u32>,
}


#[derive(Deserialize)]
struct MeteoraPoolsResponse {
    status: u16,
    pages: u32,
    data: Vec<MeteoraPool>,
}

pub async fn fetch_pools(data_folder_path: &str, is_test: bool) -> Result<HashSet<TokenInfo>> {
    let file = File::create(format!("{}/orca_pools.json", data_folder_path))
        .await
        .context("Failed to create Orca pools output file")?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(b"{\"all_pools\":[")
        .await
        .context("Failed to write JSON header")?;

    let mut first_item = true;
    let client = reqwest::Client::new();
    let mut url =
        Url::parse("https://dammv2-api.meteora.ag/pools?order=desc&limit=100")
            .context("Invalid Orca API URL")?;



    Ok(HashSet::new())
}