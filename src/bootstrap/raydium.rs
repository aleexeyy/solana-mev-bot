use reqwest::Url;
use tokio::{fs::File, io::{AsyncWriteExt, BufWriter}};
use serde::{Serialize, Deserialize};
use serde_path_to_error::deserialize;
use solana_sdk::pubkey::Pubkey;

use solana_client::nonblocking::rpc_client::RpcClient;
use super::pool_schema::{PoolInfo, TokenInfo, PoolType, DexType};
use std::collections::{HashMap, HashSet};


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
    id : Option<String>,
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

pub async fn fetch_pools() -> Result<HashSet<TokenInfo>, Box<dyn std::error::Error + Send + Sync>> {

    let file = File::create("./cached-blockchain-data/raydium_pools.json").await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"{\"all_pools\":[").await?;

    let client = reqwest::Client::new();
    let mut page = 1;
    let mut url = Url::parse("https://api-v3.raydium.io/pools/info/list?poolType=all&poolSortField=volume7d&sortType=desc&pageSize=100&page=1").unwrap();
    let mut first_item = true;
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let mut tokens = HashSet::new();
    for _ in 0..100 {

        let response = client.get(url.clone()).send().await?;
        let text = response.text().await?;

        let mut deserializer = serde_json::Deserializer::from_str(&text);
        let deserialized_response: RaydiumResponse = deserialize(&mut deserializer)
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;

        let pools = deserialized_response.data.data;

        let pool_addresses: Vec<Pubkey> = pools.iter()
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
                    pool_type: pool_type,
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
                    let json = serde_json::to_string(&generic_pool)?;
                    writer.write_all(json.as_bytes()).await?;
                    first_item = false;
                }
            }
        }

        if !deserialized_response.data.has_next_page { break; }

        page += 1;
        url.query_pairs_mut()
            .clear()
            .append_pair("poolType", "all")
            .append_pair("poolSortField", "volume7d")
            .append_pair("sortType", "desc")
            .append_pair("pageSize", "100")
            .append_pair("page", &page.to_string());

        // println!("Fetched {} pools in this batch", pools.len());
    }

    writer.write_all(b"]}").await?;
    writer.flush().await?;

    // println!("Raydium Tokens: {:?}", &tokens);

    Ok(tokens)
}


async fn fetch_vaults_batch(
    client: &RpcClient,
    pool_addresses: Vec<Pubkey>,
) -> Result<HashMap<usize, (Pubkey, Pubkey)>, Box<dyn std::error::Error + Send + Sync>> {
    // Fetch multiple accounts in one RPC call
    let accounts = client
        .get_multiple_accounts(&pool_addresses)
        .await
        .expect("Failed to fetch the Account Data");

    let mut vaults = HashMap::new();

    for (i, account_opt) in accounts.into_iter().enumerate() {
        if let Some(account) = account_opt {
            let data = account.data;
            // Defensive check
            if data.len() != 1544 {
                // eprintln!("Account {} too short, skipping", i);
                continue;
            }

            let token_a_vault = Pubkey::new_from_array(data[137..169].try_into()?);
            let token_b_vault = Pubkey::new_from_array(data[169..201].try_into()?);

            vaults.insert(i, (token_a_vault, token_b_vault));
        } else {
            eprintln!("Account {} missing (None)", i);
        }
    }

    Ok(vaults)
}