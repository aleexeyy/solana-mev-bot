use solana_client::{
   nonblocking::{rpc_client::RpcClient},
};
use anyhow::{Result, anyhow};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    account::Account, pubkey::Pubkey
};

use orca_whirlpools_client::{
    Whirlpool
};
use serde_json;
use std::{collections::HashMap, str::FromStr};
mod bootstrap;
use std::{fs::{read_dir, read_to_string}, sync::Arc, time::Instant};
use std::env;
use bootstrap::pool_schema::{PoolUpdate};

mod build_graph;
use futures::future::join_all;

use tracing::{info, warn, error};
use tracing_subscriber;


const RAYDIUM_OWNER: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
const ORCA_OWNER: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
type DecoderFn = fn(&Account) -> anyhow::Result<PoolUpdate>;

lazy_static::lazy_static! {
    static ref RAYDIUM_PUBKEY: Pubkey = Pubkey::from_str(RAYDIUM_OWNER).unwrap();
    static ref ORCA_PUBKEY: Pubkey = Pubkey::from_str(ORCA_OWNER).unwrap();

    static ref DECODERS: HashMap<Pubkey, DecoderFn> = {
        let mut m = HashMap::new();
        m.insert(*RAYDIUM_PUBKEY, decode_raydium_account as DecoderFn);
        m.insert(*ORCA_PUBKEY, decode_orca_account as DecoderFn);
        m
    };
}

fn decode_orca_account(account: &Account) -> anyhow::Result<PoolUpdate> {

    if account.data.len() != 653 {
        return Err(anyhow!("Account data has wrong length"));
    }

    let data = &account.data;
    let descriminator: [u8; 8] = data[0..8].try_into()?;

    if descriminator != [63, 149, 209, 12, 225, 128, 99, 9] {
        error!("Descriminator: {:?}", descriminator);
        return Err(anyhow!("Wrong Descriminator Found"));
    }
    // let config = Pubkey::new_from_array(data[8..40].try_into()?);
    // let bump: u8 = data[40];
    // let tick_spacing: [u8; 2] = [data[41], data[42]];
    //let fee_tier_index: [u8; 2] = [data[43], data[44]];
    //let fee_rate : [u8; 2] = [data[45], data[46]];

    //possible to do with unsafe in the future
    let liquidity: u128 = u128::from_le_bytes(data[49..65].try_into()?);
    let sqrt_price: u128 = u128::from_le_bytes(data[65..81].try_into()?);
    let current_tick_index : i32 = i32::from_le_bytes([data[81], data[82], data[83], data[84]]);
    //idea for a test: having account data and trying to decode it using this function and assert_eq it to Whilrpool decoder
    Ok(PoolUpdate { new_liquidity: liquidity, new_sqrt_price: sqrt_price, new_current_tick_index: current_tick_index })
}


fn decode_raydium_account(account: &Account) -> anyhow::Result<PoolUpdate> {
    if account.data.len() != 1544 {
        return Err(anyhow!("Account data has wrong length"));
    }

    let data = &account.data;
    let descriminator: [u8; 8] = data[0..8].try_into()?;

    if descriminator != [247, 237, 227, 245, 215, 195, 222, 70] {
        error!("Descriminator: {:?}", descriminator);
        return Err(anyhow!("Wrong Descriminator Found"));
    }

    //let bump: u8 = data[8];

    let liquidty: u128 = u128::from_le_bytes(data[237..253].try_into()?);
    let sqrt_price: u128 = u128::from_le_bytes(data[253..269].try_into()?);
    let current_tick_index : i32 = i32::from_le_bytes([data[269], data[270], data[271], data[272]]);

    Ok(PoolUpdate { new_liquidity: liquidty, new_sqrt_price: sqrt_price, new_current_tick_index: current_tick_index })
}

fn load_pools() -> anyhow::Result<Vec<Pubkey>> {

    // want all files with a .json extension
    let pool_files = Vec::from_iter(
        read_dir("./cached-blockchain-data")?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("json")),
    );

    let mut addresses = Vec::new();

    for pool_path in pool_files {
        let raw_json = read_to_string(pool_path)?;
        let deserialized: bootstrap::pool_schema::StoredPools =
            serde_json::from_str(&raw_json)?;

        addresses.extend(
            deserialized
                .all_pools
                .iter()
                .filter_map(|pool| pool.address.as_ref())
                .map(|addr| addr.parse::<Pubkey>().expect("Failed to parse")),
        );
    }

    Ok(addresses)
}


fn decode_account(account: &Account) -> anyhow::Result<PoolUpdate> {
    if let Some(decoder) = DECODERS.get(&account.owner) {
        let result: PoolUpdate = decoder(account)?;
        Ok(result)
    } else {
        info!("Unknown DEX, skipping decoding");
        Err(anyhow!("Unknown DEX"))
    }
}



#[tokio::main]
async fn main() -> Result<()> {

    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.contains(&"setup".to_string()) {
        let start = Instant::now();
        //update cached pools data
        let _ = bootstrap::update_all().await;
        let duration = start.elapsed();
        println!("Bootstrap took: {:?}", duration);
    }

    let mut graph = build_graph::build_graph()?;

    //https://api.mainnet-beta.solana.com
    //https://api.devnet.solana.com
    let client = Arc::new(RpcClient::new_with_commitment("https://api.mainnet-beta.solana.com".to_string(), CommitmentConfig::confirmed()));

    let addresses = load_pools().unwrap();
    info!("Amount of Addresses: {:?}", addresses.len());
    
    let chunks: Vec<Vec<Pubkey>> = addresses.chunks(100).map(|c| c.to_vec()).collect();
    let number_of_chunks = chunks.len();
    let start = Instant::now();

    let accounts_data: Vec<(Pubkey, Account)> = join_all(chunks.into_iter().map(|chunk| {
        let client = Arc::clone(&client);
        let chunk_clone = chunk.clone(); // local chunk
        tokio::spawn(async move { 
            let accounts = client.get_multiple_accounts(&chunk_clone).await.unwrap();
            // zip addresses with accounts, keep only Some(account)
            chunk_clone.into_iter()
                .zip(accounts.into_iter())
                .filter_map(|(address, account_opt)| account_opt.map(|acc| (address, acc)))
                .collect::<Vec<_>>()
        })
    })).await
    .into_iter()
    .filter_map(|join_result| match join_result {
        Ok(accounts) => Some(accounts), // Vec<(Pubkey, Account)>
        Err(_) => {
            warn!("A task panicked, skipping chunk");
            None
        }
    })
    .flatten()
    .collect();
    
    for (address, account) in accounts_data {
        match decode_account(&account) {
            Ok(data) => {
                if let Err(e) = graph.update_edge(&address, data) {
                    warn!("Failed to update edge {}: {:?}", address, e);
                }
            }
            Err(e) => {
                warn!("Failed to decode account {}: {:?}", address, e);
            }
        }
    }



    // info!("Testing Received Data: {:?}", pools_data);
    let duration = start.elapsed();
    info!(number_of_chunks, "Number of chunks: ");
    info!("Average Duration per Chunk: {:?}", duration.div_f32(number_of_chunks as f32));
    Ok(())

}
