use std::{env, fs::read_to_string, str::FromStr, sync::Arc, time::Instant};

use anyhow::Result;
use client::{bootstrap, decoders, deshred, get_all_pool_files, graph};
use futures::future::join_all;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tracing::{info, warn};

fn load_pools(data_folder_path: &str) -> anyhow::Result<Vec<Pubkey>> {
    let pool_files = get_all_pool_files(data_folder_path)?;

    let mut addresses = Vec::new();

    for pool_path in pool_files {
        let raw_json = read_to_string(pool_path)?;
        let deserialized: bootstrap::pool_schema::StoredPools = serde_json::from_str(&raw_json)?;

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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();

    const DATA_FOLDER: &str = "./cached-blockchain-data";

    if args.contains(&"setup".to_string()) {
        let start = Instant::now();
        //update cached pools data
        let _ = bootstrap::update_all(DATA_FOLDER, false).await;
        let duration = start.elapsed();
        println!("Bootstrap took: {:?}", duration);
    }

    deshred::deshred().await?;

    panic!("Test Panic");
    let mut graph = graph::Graph::build_graph(DATA_FOLDER)?;

    graph.build_cycles(4)?;

    //https://api.mainnet-beta.solana.com
    //https://api.devnet.solana.com
    let client = Arc::new(RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com".to_string(),
        CommitmentConfig::confirmed(),
    ));

    let addresses = load_pools(DATA_FOLDER).unwrap();
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
            chunk_clone
                .into_iter()
                .zip(accounts.into_iter())
                .filter_map(|(address, account_opt)| account_opt.map(|acc| (address, acc)))
                .collect::<Vec<_>>()
        })
    }))
    .await
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
        match decoders::decode_account(&account) {
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

    let duration = start.elapsed();
    info!(number_of_chunks, "Number of chunks: ");
    info!(
        "Average Duration per Chunk: {:?}",
        duration.div_f32(number_of_chunks as f32)
    );

    // let _ = graph.find_arbitrage_cycles()?;

    Ok(())
}
