use reqwest::Url;
use tokio::{fs::File, io::{AsyncWriteExt, BufWriter}};
use serde::{Serialize, Deserialize};
use serde_path_to_error::deserialize;
use super::pool_schema::{PoolBootstrap, TokenInfo};
use std::collections::HashSet;

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
    config: Option<String>
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

pub async fn fetch_pools() -> Result<HashSet<TokenInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::create("./cached-blockchain-data/orca_pools.json").await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"{\"all_pools\":[").await?;

    let mut first_item = true;
    let client = reqwest::Client::new();
    let mut url = Url::parse("https://api.orca.so/v2/solana/pools?sortBy=volume&sortDirection=desc").unwrap();
    let mut tokens = HashSet::new();
    for _ in 0..200 {
        let response = client.get(url.clone()).send().await?;
        let text = response.text().await?;

        let mut deserializer = serde_json::Deserializer::from_str(&text);
        let deserialized_response: OrcaPoolsResponse = deserialize(&mut deserializer)
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;

        let pools = deserialized_response.data;

        for pool in &pools {

            tokens.insert(pool.token_a.clone());
            tokens.insert(pool.token_b.clone());

            let generic_pool = PoolBootstrap {
                address: pool.address.clone(),
                fee_rate: pool.fee_rate,
                pool_type: Some("Concentrated".to_string()),
                dex: Some("Orca".to_string()),
                tick_spacing: pool.tick_spacing,
                token_a: pool.token_a.clone(),
                token_b: pool.token_b.clone(),
                token_vault_a: pool.token_vault_a.clone(),
                token_vault_b: pool.token_vault_b.clone(),
                config: pool.config.clone(),
            };

            if generic_pool.check().is_err() {
                continue;
            }

            if !first_item {
                writer.write_all(b",").await?;
            }

            let json = serde_json::to_string(&generic_pool)?;
            writer.write_all(json.as_bytes()).await?;
            first_item = false;
        }

        let next_page = match deserialized_response.meta.cursor.next {
            Some(ref n) if !n.is_empty() => n.clone(),
            _ => break,
        };

        url.query_pairs_mut()
            .clear()
            .append_pair("sortBy", "volume")
            .append_pair("sortDirection", "desc")
            .append_pair("next", &next_page);

        // println!("Fetched {} pools in this batch", pools.len());
    }

    writer.write_all(b"]}").await?;
    writer.flush().await?;

    // println!("Orca Tokens: {:?}", &tokens);

    Ok(tokens)
}
