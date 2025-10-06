use std::sync::Arc;

// use futures::StreamExt;
// use num_cpus;
use once_cell::sync::Lazy;
use solana_sdk::transaction::VersionedTransaction;
use tokio::{sync::mpsc::Receiver, task};

use crate::shred_decoders::interfaces::{DecodedTransaction, TargetTransaction};
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

static DECODERS: Lazy<[&'static dyn TargetTransaction; 6]> = Lazy::new(|| {
    [
        &RAYDIUM_V2_DECODER,
        &RAYDIUM_V3_DECODER,
        &ORCA_V3_DECODER,
        &METEORA_V3_DECODER,
        &METEORA_V2_DECODER,
        &JUPITER_V6_DECODER,
    ]
});

pub type DecodeJob = (usize, usize, usize, Arc<VersionedTransaction>, Program);

pub async fn decode_transaction(mut decode_rx: Receiver<Vec<DecodeJob>>, graph: Arc<Graph>) {
    // let stream = ReceiverStream::new(decode_rx);
    // let concurrency = num_cpus::get().max(1);
    //
    // stream.for_each_concurrent(concurrency, |batch: Vec<DecodeJob>| async move {
    while let Some(batch) = decode_rx.recv().await {
        let batch_size = batch.len();

        let t0 = std::time::Instant::now();
        let before_cpu = get_cpu_time();

        let graph = Arc::clone(&graph);

        let res = task::spawn_blocking(move || {
            let mut decoded_ok = 0usize;
            let mut decoded_err = 0usize;

            for (e_index, t_index, program_index, tx, program) in batch {
                println!("{:?}", tx);

                let idx = program.index();

                match DECODERS[idx].decode(&*tx, program_index, &graph) {
                    Ok(_decoded) => decoded_ok += 1,
                    Err(_) => decoded_err += 1,
                }
            }
            (decoded_ok, decoded_err)
        })
        .await;
        match res {
            Ok((ok, err)) => {
                let processed = ok + err;
                tracing::info!(
                    batch_size = batch_size,
                    processed = processed,
                    ok = ok,
                    err = err,
                    "decoded batch results"
                );
            }
            Err(join_err) => {
                tracing::error!("spawn_blocking panicked: {:?}", join_err);
            }
        }

        let total_wall = t0.elapsed();
        let total_cpu = get_cpu_time() - before_cpu;

        tracing::info!(
            batch_size = batch_size,
            wall = ?total_wall,
            cpu_total = ?total_cpu,
            "decode timing"
        );
    }
    // })
    // .await;
}
