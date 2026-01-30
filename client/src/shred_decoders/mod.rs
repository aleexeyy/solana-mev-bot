use std::{
    collections::HashMap,
    fs,
    sync::{Arc, OnceLock},
};

// use futures::StreamExt;
// use num_cpus;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task,
};

use crate::shred_decoders::interfaces::{
    DecodeJob, RawLookupTables, ShredEvent, TargetTransaction,
};
// use tokio_stream::wrappers::ReceiverStream;
use crate::{benchmark_tools::measure_cpu_bound::get_cpu_time, graph::Graph};

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

static DECODERS: Lazy<[&'static dyn TargetTransaction; 5]> = Lazy::new(|| {
    [
        &RAYDIUM_V2_DECODER,
        &RAYDIUM_V3_DECODER,
        &ORCA_V3_DECODER,
        &METEORA_V3_DECODER,
        &METEORA_V2_DECODER,
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

pub fn get_lookup_table_addresses(lt: &Pubkey) -> Option<&Arc<[Pubkey]>> {
    get_lookup_tables().get(lt)
}

pub async fn decode_transaction(
    mut decode_rx: Receiver<Vec<DecodeJob>>,
    simulate_tx: Sender<Vec<ShredEvent>>,
    graph: Arc<Graph>,
) {
    while let Some(batch) = decode_rx.recv().await {
        let batch_size = batch.len();

        let t0 = std::time::Instant::now();
        let before_cpu = get_cpu_time();

        let graph_ref: Arc<Graph> = Arc::clone(&graph);
        let simulate_tx_ref = simulate_tx.clone();

        let res = task::spawn_blocking(move || {
            let mut per_tx_counts = Vec::with_capacity(batch_size);

            for DecodeJob {
                slot,
                transaction_address,
                account_keys,
                lookup_tables,
                instructions,
            } in batch
            {
                let mut decoded_ok = 0usize;
                let mut decoded_err = 0usize;
                let mut decoded_tx = Vec::with_capacity(instructions.len() / 2);

                for (instruction_index, instruction) in instructions.into_iter().enumerate() {
                    let decoder_idx = instruction.program.index();
                    let instruction_index = match u8::try_from(instruction_index) {
                        Ok(v) => v,
                        Err(_) => {
                            tracing::error!(
                                "{:?}: instruction index too large: {}",
                                transaction_address,
                                instruction_index
                            );
                            decoded_err += 1;
                            continue;
                        }
                    };

                    let data_slice = &instruction.data;
                    let accounts_slice = &instruction.accounts;

                    match DECODERS[decoder_idx].decode(
                        slot,
                        transaction_address,
                        instruction_index,
                        &account_keys,
                        accounts_slice,
                        data_slice,
                        graph_ref.market_graph(),
                        &lookup_tables,
                    ) {
                        Ok(decoded_instruction) => {
                            // TODO: send directly to the engine
                            decoded_tx.push(decoded_instruction);
                            decoded_ok += 1;
                        }
                        Err(err) => {
                            tracing::error!("{:?}: {:?}", transaction_address, err);
                            tracing::error!("Instruction Data: {:?}", &instruction.data);
                            decoded_err += 1;
                        }
                    }
                }

                if !decoded_tx.is_empty() {
                    let simulate_tx_clone = simulate_tx_ref.clone();
                    tokio::spawn(async move {
                        if let Err(e) = simulate_tx_clone.send(decoded_tx).await {
                            tracing::debug!("simulate_tx closed: {:?}", e);
                        }
                    });
                }

                per_tx_counts.push((transaction_address, decoded_ok, decoded_err));
            }

            per_tx_counts
        })
        .await;

        match res {
            Ok(per_tx_counts) => {
                for (_, ok, err) in per_tx_counts.into_iter() {
                    let processed = ok + err;
                    tracing::debug!(
                        processed = processed,
                        ok = ok,
                        err = err,
                        "decoded transaction results"
                    );
                }
            }
            Err(err) => {
                tracing::error!("Error during decoding: {:?}", err);
            }
        }

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
