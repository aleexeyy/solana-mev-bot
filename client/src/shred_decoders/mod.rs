use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use num_cpus;
use once_cell::sync::Lazy;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use tokio::{sync::mpsc::Receiver, task};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    benchmark_tools::measure_cpu_bound::get_cpu_time, graph::Graph, target_dexes::Program,
};

mod jupiter_v6;
mod meteora_dlmm;
mod meteora_v2;
mod meteora_v3;
mod orca_v3;
mod raydium_v2;
mod raydium_v3;

pub trait TargetTransaction: Sync + Send {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
    ) -> Result<DecodedTransaction>;

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;

    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;
}

// TODO: some DEXes write token_in and token_out, others just write if the swap is_direct, handle both cases, some do it without providing tokens
#[derive(Debug)]
pub enum OperationType {
    SwapExactInput {
        amount_in: u64,
        minimum_amount_out: u64,
        sqrt_price_limit: u128,
    },
    SwapExactOutput {
        amount_out: u64,
        maximum_amount_in: u64,
        sqrt_price_limit: u128,
    },
    AddLiquidity {
        add_amount_a: u64,
        add_amount_b: u64,
    },
    RemoveLiquidity {
        remove_amount_a: u64,
        remove_amount_b: u64,
    },
}

#[derive(Debug)]
pub struct DecodedInstruction {
    pool_address: Pubkey,
    token_in_address: Pubkey,  // also input token
    token_out_address: Pubkey, // also output token
    token_in_vault: Pubkey,
    token_out_vault: Pubkey,
    operation_type: OperationType, // TODO: Check Operation Type and Adjust the Sign of change liquidity based on Operation Type
}

#[derive(Debug)]
pub struct DecodedTransaction {
    instructions: Vec<DecodedInstruction>,
}

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

pub fn decode_transaction(
    program: Program,
    transaction: &VersionedTransaction,
    program_index: usize,
) -> Result<DecodedTransaction> {
    let idx = program.index();
    DECODERS[idx].decode(transaction, program_index)
}

pub type DecodeJob = (usize, usize, usize, Arc<VersionedTransaction>, Program);

pub async fn test_decode(mut decode_rx: Receiver<Vec<DecodeJob>>, graph: &Arc<Graph>) {
    // let stream = ReceiverStream::new(decode_rx);
    // let concurrency = num_cpus::get().max(1);
    //
    // stream
    //     .for_each_concurrent(concurrency, |batch: Vec<DecodeJob>| async move {
    while let Some(batch) = decode_rx.recv().await {
        let batch_size = batch.len();

        let t0 = std::time::Instant::now();
        let before_cpu = get_cpu_time();

        let res = task::spawn_blocking(move || {
            let mut decoded_ok = 0usize;
            let mut decoded_err = 0usize;

            for (e_index, t_index, program_index, tx, program) in batch {
                // println!("{:?}", tx);

                match decode_transaction(program, &*tx, program_index) {
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
