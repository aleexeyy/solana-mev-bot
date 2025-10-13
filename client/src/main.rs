use std::{env, fs::read_to_string, sync::Arc, time::Instant};

use anyhow::Result;
use arc_swap::ArcSwap;
use client::{
    bootstrap, get_all_pool_files, get_shreds, graph, shred_decoders,
    shred_decoders::{get_lookup_tables, interfaces::DecodeJob},
};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;

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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();

    const DATA_FOLDER: &str = "./client/cached-blockchain-data";

    if args.contains(&"setup".to_string()) {
        let start = Instant::now();
        //update cached pools data
        let _ = bootstrap::update_all(DATA_FOLDER, false).await;
        let duration = start.elapsed();
        println!("Bootstrap took: {:?}", duration);
    }
    // let test_owner = Pubkey::from_str("DxZ2rMQfnT52sW4hEmADh9PULKofcCNwp9KeG4SQ77aB").unwrap();
    // let test_token = Pubkey::from_str("F9a5yraZ92WHRBaA3CMnRu2R26y5rCVjHzwGuoD8EYSj").unwrap();
    // let token_account =
    //     MeteoraV3TargetTransaction::find_token_account_address(&test_owner, &test_token);
    //
    // println!("Received Token Account Address: {:?}", token_account);
    //
    // panic!("Test Panic!!!");
    let mut graph = graph::Graph::build_graph(DATA_FOLDER)?;
    graph.build_cycles(4)?;

    let _ = get_lookup_tables();

    let graph = Arc::new(ArcSwap::new(Arc::new(graph)));

    let (decode_tx, decode_rx) = mpsc::channel::<Vec<DecodeJob>>(128);

    let graph_for_decoder = Arc::clone(&graph);
    // let graph_for_arbitrage = Arc::clone(&graph);

    tokio::spawn(async move {
        let snapshot = graph_for_decoder.load_full();
        shred_decoders::decode_transaction(decode_rx, snapshot).await;
    });

    tokio::spawn(async move {
        let decode_tx = decode_tx.clone();
        if let Err(e) = get_shreds::deshred(decode_tx).await {
            eprintln!("Shredstream error: {:?}", e);
        }
    });

    //TODO: Patcher, using WebSocket fetch blocks and apply updates
    // let graph_for_patcher = Arc::clone(&graph);
    // tokio::spawn(async move {
    //     loop {
    //         let mut new_graph = (*graph_for_patcher.load_full()).clone_mutable();
    //         new_graph.update_something().unwrap();
    //         graph_for_patcher.store(Arc::new(new_graph));
    //         tokio::time::sleep(Duration::from_millis(400)).await;
    //     }
    // });

    // let client = Arc::new(RpcClient::new_with_commitment(
    //     "https://api.mainnet-beta.solana.com".to_string(),
    //     CommitmentConfig::confirmed(),
    // ));
    //
    // let addresses = load_pools(DATA_FOLDER).unwrap();
    // info!("Amount of Addresses: {:?}", addresses.len());
    //
    // let chunks: Vec<Vec<Pubkey>> = addresses.chunks(100).map(|c| c.to_vec()).collect();
    // let number_of_chunks = chunks.len();
    // let start = Instant::now();
    //
    // let accounts_data: Vec<(Pubkey, Account)> = join_all(chunks.into_iter().map(|chunk| {
    //     let client = Arc::clone(&client);
    //     let chunk_clone = chunk.clone(); // local chunk
    //     tokio::spawn(async move {
    //         let accounts = client.get_multiple_accounts(&chunk_clone).await.unwrap();
    //         // zip addresses with accounts, keep only Some(account)
    //         chunk_clone
    //             .into_iter()
    //             .zip(accounts.into_iter())
    //             .filter_map(|(address, account_opt)| account_opt.map(|acc| (address, acc)))
    //             .collect::<Vec<_>>()
    //     })
    // }))
    // .await
    // .into_iter()
    // .filter_map(|join_result| match join_result {
    //     Ok(accounts) => Some(accounts), // Vec<(Pubkey, Account)>
    //     Err(_) => {
    //         warn!("A task panicked, skipping chunk");
    //         None
    //     }
    // })
    // .flatten()
    // .collect();
    //
    // for (address, account) in accounts_data {
    //     match decoders::decode_account(&account) {
    //         Ok(data) => {
    //             if let Err(e) = graph.update_edge(&address, data) {
    //                 warn!("Failed to update edge {}: {:?}", address, e);
    //             }
    //         }
    //         Err(e) => {
    //             warn!("Failed to decode account {}: {:?}", address, e);
    //         }
    //     }
    // }
    //
    // let duration = start.elapsed();
    // info!(number_of_chunks, "Number of chunks: ");
    // info!(
    //     "Average Duration per Chunk: {:?}",
    //     duration.div_f32(number_of_chunks as f32)
    // );

    tokio::signal::ctrl_c().await?;
    println!("Exiting...");

    // let _ = graph.find_arbitrage_cycles()?;

    Ok(())
}
