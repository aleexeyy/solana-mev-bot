use std::{env, sync::Arc, vec};

use anyhow::Result;
use dotenvy::dotenv;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use solana_entry::entry::Entry;
use solana_sdk::{pubkey::Pubkey, transaction::TransactionVersion};
use tokio::sync::mpsc::Sender;

use crate::{
    shred_decoders::interfaces::{DecodeJob, InstructionData, ReducedLookupTable},
    target_dexes::match_program,
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

        if !target_shreds.is_empty() && decode_tx.send(target_shreds).await.is_err() {
            break;
        };
    }
    Ok(())
}
pub fn filter_by_programs(entries: &[Entry]) -> Vec<DecodeJob> {
    let mut jobs = Vec::new();

    for entry in entries.iter() {
        for tx in entry.transactions.iter() {
            let account_keys: Arc<[Pubkey]> = Arc::from(tx.message.static_account_keys());
            let mut instructions = Vec::new();

            for instruction in tx.message.instructions() {
                let program_index = instruction.program_id_index as usize;
                let program_key = &account_keys[program_index];

                if let Some(program) = match_program(program_key) {
                    instructions.push(InstructionData {
                        program,
                        accounts: Arc::from(instruction.accounts.as_slice()),
                        data: Arc::from(instruction.data.as_slice()),
                    });
                }

                // if tx.version() == TransactionVersion::Number(0) {
                //     println!("Transaction: {:?}", tx);
                // }
            }

            if !instructions.is_empty() {
                if let Some(lookup_tables) = tx.message.address_table_lookups() {
                    jobs.push(DecodeJob {
                        transaction_address: tx.signatures[0],
                        account_keys,
                        lookup_tables: Arc::new(
                            lookup_tables
                                .iter()
                                .map(|table| {
                                    let mut combined = Vec::with_capacity(
                                        table.writable_indexes.len() + table.readonly_indexes.len(),
                                    );
                                    combined.extend_from_slice(&table.writable_indexes);
                                    combined.extend_from_slice(&table.readonly_indexes);
                                    let indexes: Arc<[u8]> = Arc::from(combined);
                                    ReducedLookupTable {
                                        account_key: table.account_key,
                                        indexes,
                                    }
                                })
                                .collect(),
                        ),
                        instructions,
                    });
                } else {
                    jobs.push(DecodeJob {
                        transaction_address: tx.signatures[0],
                        account_keys,
                        lookup_tables: Arc::new(vec![ReducedLookupTable::default()]),
                        instructions,
                    });
                }
            }
        }
    }

    jobs
}
