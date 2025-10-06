use std::{env, sync::Arc};

use anyhow::Result;
use dotenvy::dotenv;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use solana_entry::entry::Entry;
use solana_sdk::{
    pubkey::Pubkey,
    transaction::{TransactionVersion, VersionedTransaction},
};
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    benchmark_tools::{measure_cpu_bound, measure_cpu_bound::get_cpu_time},
    graph::Graph,
    target_dexes::{Program, match_program},
    transaction_decoders,
};

pub async fn deshred(graph: Arc<RwLock<Graph>>) -> Result<()> {
    dotenv().ok();
    let shredstream_address = env::var("SHREDSTREAM_ADDRESS")?;
    let mut client = ShredstreamProxyClient::connect(shredstream_address).await?;

    let mut stream = client
        .subscribe_entries(SubscribeEntriesRequest {})
        .await?
        .into_inner();

    while let Some(slot_entry) = stream.message().await? {
        let entries =
            match bincode::deserialize::<Vec<solana_entry::entry::Entry>>(&slot_entry.entries) {
                Ok(e) => e,
                Err(e) => {
                    println!("Deserialization failed with err: {e}");
                    continue;
                }
            };

        filter_by_programs(entries.as_slice())?;
    }
    Ok(())
}
pub fn filter_by_programs(entries: &[Entry]) -> Result<()> {
    let t0 = std::time::Instant::now();
    let before_cpu = get_cpu_time();

    let matches: Vec<(usize, usize, usize, &VersionedTransaction, Program)> = entries
        .iter()
        .enumerate()
        .flat_map(|(e_index, entry)| {
            entry
                .transactions
                .iter()
                .enumerate()
                .filter_map(move |(t_index, tx)| {
                    let mut first_non_jupiter: Option<(usize, Program)> = None;

                    for (program_index, account_key) in
                        tx.message.static_account_keys().iter().enumerate()
                    {
                        if let Some(program) = match_program(account_key) {
                            if program == Program::Jupiter {
                                return Some((e_index, t_index, program_index, tx, program));
                            }

                            if first_non_jupiter.is_none() {
                                first_non_jupiter = Some((program_index, program));
                            }
                        }
                    }
                    first_non_jupiter.map(|(program_index, program)| {
                        (e_index, t_index, program_index, tx, program)
                    })
                })
        })
        .collect();

    let filter_wall = t0.elapsed();

    let mut decoded_ok = 0usize;
    let mut decoded_err = 0usize;

    for (e_index, t_index, program_index, tx, program) in &matches {
        println!("{:?}", tx);

        if let Ok(decoded_transaction) =
            transaction_decoders::decode_transaction(*program, tx, *program_index)
        {
            decoded_ok += 1;
            // println!("decoded transaction: {:?}", decoded_transaction);
        } else {
            decoded_err += 1;
            // println!("Transaction decode failed with err");
        }
        println!("Match at {}:{}", e_index, t_index);
        println!("Program: {:?}", program);
        println!(
            "------------------------------------------------------------------------------------------"
        );
    }

    let total_wall = t0.elapsed();
    let total_cpu = get_cpu_time() - before_cpu;

    info!(
        "entries={} matches={} decoded_ok={} decoded_err={} filter={:?} decode={:?} cpu_total={:?}",
        entries.len(),
        matches.len(),
        decoded_ok,
        decoded_err,
        filter_wall,
        total_wall - filter_wall,
        total_cpu
    );
    println!();

    Ok(())
}
