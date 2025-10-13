use std::{
    collections::HashMap,
    fs,
    sync::{Arc, OnceLock},
};

// use futures::StreamExt;
// use num_cpus;
use once_cell::sync::Lazy;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use tokio::{sync::mpsc::Receiver, task};

use crate::shred_decoders::interfaces::{DecodeJob, RawLookupTables, TargetTransaction};
// use tokio_stream::wrappers::ReceiverStream;
use crate::{
    benchmark_tools::measure_cpu_bound::get_cpu_time, graph::Graph, target_dexes::Program,
};

mod jupiter_v6;
mod meteora_dlmm;
mod meteora_v2;
pub mod meteora_v3;
mod orca_v3;
mod raydium_v2;
mod raydium_v3;

pub mod interfaces;
pub mod utils;

pub static RAYDIUM_V2_DECODER: raydium_v2::RaydiumV2TargetTransaction =
    raydium_v2::RaydiumV2TargetTransaction;
pub static RAYDIUM_V3_DECODER: raydium_v3::RaydiumV3TargetTransaction =
    raydium_v3::RaydiumV3TargetTransaction;
pub static ORCA_V3_DECODER: orca_v3::OrcaV3TargetTransaction = orca_v3::OrcaV3TargetTransaction;
pub static METEORA_V3_DECODER: meteora_v3::MeteoraV3TargetTransaction =
    meteora_v3::MeteoraV3TargetTransaction;
pub static METEORA_V2_DECODER: meteora_v2::MeteoraV2TargetTransaction =
    meteora_v2::MeteoraV2TargetTransaction;
pub static JUPITER_V6_DECODER: jupiter_v6::JupiterV6TargetTransaction =
    jupiter_v6::JupiterV6TargetTransaction;

static DECODERS: Lazy<[&'static dyn TargetTransaction; 5]> = Lazy::new(|| {
    [
        &RAYDIUM_V2_DECODER,
        &RAYDIUM_V3_DECODER,
        &ORCA_V3_DECODER,
        &METEORA_V3_DECODER,
        &METEORA_V2_DECODER,
        // &JUPITER_V6_DECODER,
    ]
});

static LOOKUP_TABLES: OnceLock<HashMap<Pubkey, Arc<[Pubkey]>>> = OnceLock::new();

pub fn get_lookup_tables() -> &'static HashMap<Pubkey, Arc<[Pubkey]>> {
    LOOKUP_TABLES.get_or_init(|| {
        // Adjust the path to your JSON file
        let json_bytes =
            fs::read("./client/src/shred_decoders/alts.json").expect("Failed to read alts.json");

        let raw: RawLookupTables =
            serde_json::from_slice(&json_bytes).expect("Failed to parse lookup tables JSON");

        // Convert raw map into optimized map
        let mut map = HashMap::with_capacity(raw.len());
        for (lt_addr_str, addrs_str_vec) in raw {
            let lt_addr = lt_addr_str
                .parse::<Pubkey>()
                .unwrap_or_else(|e| panic!("Bad lookup table address '{}': {:?}", lt_addr_str, e));

            // Convert each stored string to Pubkey
            let mut v: Vec<Pubkey> = Vec::with_capacity(addrs_str_vec.len());
            for s in addrs_str_vec {
                let pk = s.parse::<Pubkey>().unwrap_or_else(|e| {
                    panic!(
                        "Bad stored address in lookup table {}: '{}' -> {:?}",
                        lt_addr_str, s, e
                    )
                });
                v.push(pk);
            }

            // Convert Vec<Pubkey> to Arc<[Pubkey]>
            let arc_slice: Arc<[Pubkey]> = Arc::from(v.into_boxed_slice());
            map.insert(lt_addr, arc_slice);
        }

        map
    })
}

pub fn lookup_table_addresses(lt: &Pubkey) -> Option<&Arc<[Pubkey]>> {
    get_lookup_tables().get(lt)
}

//TODO: implement decoding logic also for adding and removing liquidity(sometimes they do create an arbitrage opportunity)
//TODO: implement Lookup table check in case of index out of bounds
pub async fn decode_transaction(mut decode_rx: Receiver<Vec<DecodeJob>>, graph: Arc<Graph>) {
    while let Some(batch) = decode_rx.recv().await {
        let batch_size = batch.len();

        let t0 = std::time::Instant::now();
        let before_cpu = get_cpu_time();

        let graph = Arc::clone(&graph);

        let res = task::spawn_blocking(move || {
            let mut per_tx_counts = Vec::with_capacity(batch.len());

            for DecodeJob {
                transaction_address,
                account_keys,
                lookup_tables,
                instructions,
            } in batch
            {
                let mut decoded_ok = 0usize;
                let mut decoded_err = 0usize;

                for instruction in instructions {
                    let idx = instruction.program.index();

                    let data_slice: &[u8] = &*instruction.data;
                    let accounts_slice: &[u8] = &*instruction.accounts;

                    match DECODERS[idx].decode(&account_keys, accounts_slice, data_slice, &graph) {
                        Ok(_) => decoded_ok += 1,
                        Err(err) => {
                            tracing::error!("{:?}: {:?}", transaction_address, err);
                            tracing::error!("Instruction Data: {:?}", &instruction.data);
                            decoded_err += 1
                        }
                    }
                }

                per_tx_counts.push((transaction_address, decoded_ok, decoded_err));
            }

            per_tx_counts
        })
        .await;

        // match res {
        //     Ok(per_tx_counts) => {
        //         for (tx_address, ok, err) in per_tx_counts.into_iter() {
        //             let processed = ok + err;
        //             tracing::info!(
        //                 processed = processed,
        //                 ok = ok,
        //                 err = err,
        //                 "decoded transaction results"
        //             );
        //         }
        //     }
        //     Err(err) => {
        //         tracing::error!("Error during decoding: {:?}", err);
        //     }
        // }

        let total_wall = t0.elapsed();
        let total_cpu = get_cpu_time() - before_cpu;

        tracing::info!(
            batch_size = batch_size,
            wall = ?total_wall,
            cpu_total = ?total_cpu,
            "decode batch timing"
        );
    }
}
