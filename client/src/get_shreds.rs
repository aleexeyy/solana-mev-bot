use std::str::FromStr;

use anyhow::Result;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use solana_entry::entry::Entry;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::{
    target_dexes::{Program, match_program},
    transaction_decoders,
};

pub async fn deshred() -> Result<()> {
    let mut client = ShredstreamProxyClient::connect("http://88.99.142.79:50051").await?;

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
        println!(
            "slot {}, entries: {}, transactions: {}",
            slot_entry.slot,
            entries.len(),
            entries.iter().map(|e| e.transactions.len()).sum::<usize>()
        );

        let _ = filter_by_programs(entries.as_slice())?;
    }
    Ok(())
}

pub fn filter_by_programs(
    entries: &[Entry],
) -> Result<Vec<(usize, usize, usize, &VersionedTransaction, Program)>> {
    // Collect all matching transactions; small linear scan per tx over its account keys.
    let matches: Vec<(usize, usize, usize, &VersionedTransaction, Program)> = entries
        .iter()
        .enumerate()
        .flat_map(|(e_index, entry)| {
            // move closure so e_index is copied into it; tx is borrowed
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

    for (e_index, t_index, program_index, tx, program) in &matches {
        println!("{:?}", tx);
        if let Ok(decoded_transaction) =
            transaction_decoders::decode_transaction(*program, tx, *program_index)
        {
            println!("decoded transaction: {:?}", decoded_transaction);
        } else {
            println!("Transaction decode failed with err");
        }
        // println!("Match at {}:{}", e_index, t_index);
        println!("Program: {:?}", program);
        println!(
            "------------------------------------------------------------------------------------------"
        );
    }

    Ok(matches)
}
