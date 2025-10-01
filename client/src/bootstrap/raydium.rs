use super::pool_schema::{DexType, PoolInfo, PoolType, TokenInfo};
use anyhow::{Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RaydiumPool {
    id: Option<String>,
    #[serde(rename = "type")]
    pool_type: Option<String>,
    #[serde(rename = "mintA")]
    token_a: RaydiumToken,
    #[serde(rename = "mintB")]
    token_b: RaydiumToken,
    config: Option<RaydiumConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RaydiumToken {
    address: Option<String>,
    #[serde(rename = "programId")]
    program_id: Option<String>,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RaydiumConfig {
    id: Option<String>,
    #[serde(rename = "tickSpacing")]
    tick_spacing: Option<u64>,
    #[serde(rename = "tradeFeeRate")]
    trade_fee_rate: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumData {
    data: Vec<RaydiumPool>,
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumResponse {
    data: RaydiumData,
}

pub async fn fetch_pools(data_folder_path: &str, is_test: bool) -> Result<HashSet<TokenInfo>> {
    let file = File::create(format!("{}/raydium_pools.json", data_folder_path))
        .await
        .context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(b"{\"all_pools\":[")
        .await
        .context("Failed to write JSON header")?;

    let client = reqwest::Client::new();
    let mut page = 1;
    let mut url = Url::parse("https://api-v3.raydium.io/pools/info/list?poolType=all&poolSortField=volume7d&sortType=desc&pageSize=100&page=1")
        .context("Invalid Raydium URL")?;
    let mut first_item = true;
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let mut tokens = HashSet::new();

    let max_iterations: usize = match is_test {
        true => 1,
        false => 5, // change for production
    };

    //100 per page
    for _ in 0..max_iterations {
        let response = client
            .get(url.clone())
            .send()
            .await
            .context("HTTP request failed")?;
        let text = response
            .text()
            .await
            .context("Failed to read response body")?;

        let mut deserializer = Deserializer::from_str(&text);
        let deserialized_response: RaydiumResponse =
            serde_path_to_error::deserialize(&mut deserializer)
                .context("Failed to deserialize Raydium response")?;

        let pools = deserialized_response.data.data;
        let pool_addresses: Vec<Pubkey> = pools
            .iter()
            .filter_map(|pool| pool.id.as_ref()?.parse().ok())
            .collect();

        let vaults = fetch_vaults_batch(&rpc_client, pool_addresses).await?;

        for (pool_index, pool) in pools.iter().enumerate() {
            if let Some((token_a_vault, token_b_vault)) = vaults.get(&pool_index) {
                tokens.insert(TokenInfo {
                    address: pool.token_a.address.clone(),
                    decimals: pool.token_a.decimals,
                    name: pool.token_a.name.clone(),
                    symbol: pool.token_a.symbol.clone(),
                });
                tokens.insert(TokenInfo {
                    address: pool.token_b.address.clone(),
                    decimals: pool.token_b.decimals,
                    name: pool.token_b.name.clone(),
                    symbol: pool.token_b.symbol.clone(),
                });

                let pool_type = match pool.pool_type.as_deref() {
                    Some("Concentrated") => Some(PoolType::Concentrated),
                    Some("Standard") => Some(PoolType::Standard),
                    _ => None,
                };

                let generic_pool = PoolInfo {
                    address: pool.id.clone(),
                    fee_rate: pool.config.as_ref().and_then(|c| c.trade_fee_rate),
                    pool_type,
                    dex: Some(DexType::Raydium),
                    tick_spacing: pool.config.as_ref().and_then(|c| c.tick_spacing),
                    token_a: Some(TokenInfo {
                        address: pool.token_a.address.clone(),
                        decimals: pool.token_a.decimals,
                        name: pool.token_a.name.clone(),
                        symbol: pool.token_a.symbol.clone(),
                    }),
                    token_b: Some(TokenInfo {
                        address: pool.token_b.address.clone(),
                        decimals: pool.token_b.decimals,
                        name: pool.token_b.name.clone(),
                        symbol: pool.token_b.symbol.clone(),
                    }),
                    token_vault_a: Some(token_a_vault.to_string()),
                    token_vault_b: Some(token_b_vault.to_string()),
                    config: pool.config.as_ref().and_then(|c| c.id.clone()),
                };

                if generic_pool.check().is_ok() {
                    if !first_item {
                        writer.write_all(b",").await?;
                    }
                    let json = serde_json::to_string(&generic_pool)
                        .context("Failed to serialize PoolInfo")?;
                    writer
                        .write_all(json.as_bytes())
                        .await
                        .context("Failed to write pool JSON")?;
                    first_item = false;
                }
            }
        }

        if !deserialized_response.data.has_next_page {
            break;
        }

        page += 1;
        url.query_pairs_mut()
            .clear()
            .append_pair("poolType", "all")
            .append_pair("poolSortField", "volume7d")
            .append_pair("sortType", "desc")
            .append_pair("pageSize", "100")
            .append_pair("page", &page.to_string());
    }

    writer.write_all(b"]}").await?;
    writer.flush().await?;

    Ok(tokens)
}

async fn fetch_vaults_batch(
    client: &RpcClient,
    pool_addresses: Vec<Pubkey>,
) -> Result<HashMap<usize, (Pubkey, Pubkey)>> {
    let accounts = client
        .get_multiple_accounts(&pool_addresses)
        .await
        .context("Failed to fetch vault accounts")?;

    let mut vaults = HashMap::new();

    for (i, account_opt) in accounts.into_iter().enumerate() {
        if let Some(account) = account_opt {
            let data = account.data;
            if data.len() != 1544 {
                continue;
            }

            let token_a_vault = Pubkey::new_from_array(
                data[137..169]
                    .try_into()
                    .context("Failed to parse token_a_vault")?,
            );
            let token_b_vault = Pubkey::new_from_array(
                data[169..201]
                    .try_into()
                    .context("Failed to parse token_b_vault")?,
            );

            vaults.insert(i, (token_a_vault, token_b_vault));
        } else {
            eprintln!("Account {} missing (None)", i);
        }
    }

    Ok(vaults)
}
