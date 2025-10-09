use std::{env, sync::Arc};

use anyhow::Result;
use dotenvy::dotenv;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use solana_entry::entry::Entry;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc::Sender;

use crate::{
    shred_decoders::{DecodeJob, InstructionData},
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
            let account_keys: Arc<[Pubkey]> = Arc::from(tx.message.static_account_keys().to_vec());
            let mut instructions = Vec::new();

            for instruction in tx.message.instructions() {
                let program_index = instruction.program_id_index as usize;
                let program_key = &account_keys[program_index];

                if let Some(program) = match_program(program_key) {
                    instructions.push(InstructionData {
                        program,
                        accounts: instruction.accounts.clone(),
                        data: instruction.data.clone(),
                    });
                }
            }

            if !instructions.is_empty() {
                jobs.push(DecodeJob {
                    transaction_address: tx.signatures[0],
                    account_keys,
                    instructions,
                });
            }
        }
    }

    jobs
}

//
// pub fn filter_by_programs(entries: &[Entry]) -> Vec<DecodeJob> {
//     let matches: Vec<DecodeJob> = entries
//         .iter()
//         .enumerate()
//         .flat_map(|(e_index, entry)| {
//             entry
//                 .transactions
//                 .iter()
//                 .enumerate()
//                 .filter_map(move |(t_index, tx)| {
//                     let mut first_non_jupiter: Option<(usize, Program)> = None;
//
//                     for (program_index, account_key) in
//                         tx.message.static_account_keys().iter().enumerate()
//                     {
//                         if let Some(program) = match_program(account_key) {
//                             if program == Program::Jupiter {
//                                 return Some((
//                                     e_index,
//                                     t_index,
//                                     program_index,
//                                     Arc::new(tx.clone()),
//                                     program,
//                                 ));
//                             }
//
//                             if first_non_jupiter.is_none() {
//                                 first_non_jupiter = Some((program_index, program));
//                             }
//                         }
//                     }
//                     first_non_jupiter.map(|(program_index, program)| {
//                         (
//                             e_index,
//                             t_index,
//                             program_index,
//                             Arc::new(tx.clone()),
//                             program,
//                         )
//                     })
//                 })
//         })
//         .collect();
//
//     matches
// }
