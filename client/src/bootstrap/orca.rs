use std::collections::HashSet;

use anyhow::{Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};

use super::pool_schema::{DexType, PoolInfo, PoolType, TokenInfo};
#[derive(Debug, Serialize, Deserialize)]
struct OrcaPool {
    address: Option<String>,
    #[serde(rename = "feeRate")]
    fee_rate: Option<u32>,
    #[serde(rename = "poolType")]
    pool_type: Option<String>,
    #[serde(rename = "tickSpacing")]
    tick_spacing: Option<u64>,
    #[serde(rename = "tokenA")]
    token_a: TokenInfo,
    #[serde(rename = "tokenB")]
    token_b: TokenInfo,
    #[serde(rename = "tokenVaultA")]
    token_vault_a: Option<String>,
    #[serde(rename = "tokenVaultB")]
    token_vault_b: Option<String>,
    #[serde(rename = "whirlpoolsConfig")]
    config: Option<String>,
}

#[derive(Deserialize)]
struct OrcaPoolsResponse {
    data: Vec<OrcaPool>,
    meta: Meta,
}

#[derive(Debug, Deserialize)]
struct Meta {
    cursor: Cursor,
}

#[derive(Debug, Deserialize)]
struct Cursor {
    next: Option<String>,
    _previous: Option<String>,
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
        Url::parse("https://api.orca.so/v2/solana/pools?sortBy=volume24h&sortDirection=desc")
            .context("Invalid Orca API URL")?;
    let mut tokens = HashSet::new();

    let max_iterations: usize = match is_test {
        true => 1,
        false => 10, // change for production
    };

    // 50 per page
    for _ in 0..max_iterations {
        let response = client
            .get(url.clone())
            .send()
            .await
            .context("HTTP request to Orca API failed")?;

        let text = response
            .text()
            .await
            .context("Failed to read Orca API response body")?;

        let mut deserializer = Deserializer::from_str(&text);
        let deserialized_response: OrcaPoolsResponse =
            serde_path_to_error::deserialize(&mut deserializer)
                .context("Failed to deserialize Orca response")?;

        let pools = deserialized_response.data;

        for pool in &pools {
            tokens.insert(pool.token_a.clone());
            tokens.insert(pool.token_b.clone());

            let generic_pool = PoolInfo {
                address: pool.address.clone(),
                fee_rate: pool.fee_rate,
                pool_type: Some(PoolType::Concentrated),
                dex: Some(DexType::Orca),
                tick_spacing: pool.tick_spacing,
                token_a: Some(pool.token_a.clone()),
                token_b: Some(pool.token_b.clone()),
                token_vault_a: pool.token_vault_a.clone(),
                token_vault_b: pool.token_vault_b.clone(),
                config: pool.config.clone(),
            };

            if generic_pool.check().is_err() {
                continue;
            }

            if !first_item {
                writer
                    .write_all(b",")
                    .await
                    .context("Failed to write JSON separator")?;
            }

            let json =
                serde_json::to_string(&generic_pool).context("Failed to serialize PoolInfo")?;

            writer
                .write_all(json.as_bytes())
                .await
                .context("Failed to write pool JSON")?;

            first_item = false;
        }

        let next_page = match deserialized_response.meta.cursor.next {
            Some(ref n) if !n.is_empty() => n.clone(),
            _ => break,
        };

        url.query_pairs_mut()
            .clear()
            .append_pair("sortBy", "volume24h")
            .append_pair("sortDirection", "desc")
            .append_pair("next", &next_page);
    }

    writer
        .write_all(b"]}")
        .await
        .context("Failed to write JSON footer")?;
    writer.flush().await.context("Failed to flush writer")?;

    Ok(tokens)
}
