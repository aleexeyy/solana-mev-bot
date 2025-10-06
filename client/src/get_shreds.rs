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
use tokio::sync::{RwLock, mpsc::Sender};
use tracing::info;

use crate::{
    benchmark_tools::{measure_cpu_bound, measure_cpu_bound::get_cpu_time},
    graph::Graph,
    shred_decoders,
    shred_decoders::DecodeJob,
    target_dexes::{Program, match_program},
};

pub async fn deshred(decode_tx: Sender<Vec<DecodeJob>>) -> Result<()> {
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

        let target_shreds = filter_by_programs(entries.as_slice());

        if !target_shreds.is_empty() {
            if decode_tx.send(target_shreds).await.is_err() {
                break;
            };
        }
    }
    Ok(())
}
pub fn filter_by_programs(entries: &[Entry]) -> Vec<DecodeJob> {
    let matches: Vec<DecodeJob> = entries
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
                                return Some((
                                    e_index,
                                    t_index,
                                    program_index,
                                    Arc::new(tx.clone()),
                                    program,
                                ));
                            }

                            if first_non_jupiter.is_none() {
                                first_non_jupiter = Some((program_index, program));
                            }
                        }
                    }
                    first_non_jupiter.map(|(program_index, program)| {
                        (
                            e_index,
                            t_index,
                            program_index,
                            Arc::new(tx.clone()),
                            program,
                        )
                    })
                })
        })
        .collect();

    matches
}
