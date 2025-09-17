use solana_client::{
   nonblocking::{rpc_client::RpcClient},
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    account::Account, pubkey::{Pubkey}
};

use orca_whirlpools_client::{
    Whirlpool
};
use serde_json;

mod bootstrap;
use std::time::Instant;
use std::env;

mod build_graph;

trait WhirlpoolExt {
    fn price(&self) -> f64;
}

impl WhirlpoolExt for Whirlpool {
    fn price(&self) -> f64 {
        let sqrt_price_x64 = self.sqrt_price;
        let num = (sqrt_price_x64 as f64) / (1u128 << 64) as f64;
        num.powi(2)
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args: Vec<String> = env::args().collect();
    if args.contains(&"setup".to_string()) {
        let start = Instant::now();
        //update cached pools data
        let _ = bootstrap::update_all().await;
        let duration = start.elapsed();
        println!("Bootstrap took: {:?}", duration);
    }

    let _ = build_graph::build_graph();

    return Ok(());

    //https://api.mainnet-beta.solana.com
    //https://api.devnet.solana.com
    let client = RpcClient::new_with_commitment("https://api.mainnet-beta.solana.com".to_string(), CommitmentConfig::confirmed());


    let raw_json = tokio::fs::read_to_string("./cached-pools-data/orca_pools.json").await.expect("Failed to open the file");

    let deserialized_file : serde_json::Value = serde_json::from_str(&raw_json).expect("JSON was not well formatted");
    // println!("{:?}", deserialized_file);

    let addresses: Vec<Pubkey> = deserialized_file.get("all_pools").expect("No all_tokens found")
        .as_array().expect("all_tokens is not an array").iter()
        .map(|i| i.get("address").expect("Not addr field found")
        .as_str().expect("address is not a string")
        .parse::<Pubkey>().expect("Failed to parse"))
        .collect();

    // println!("Amount of Addresses: {:?}", addresses.len());
    // let mut update_accounts = vec![];
    let mut chunked_address : Vec<Vec<Pubkey>> = [].to_vec();
    for chunk in addresses.chunks(100) {
        chunked_address.push(chunk.to_vec());
        let accounts: Vec<Account> = client.get_multiple_accounts(chunk).await.unwrap().into_iter().map(|i| i.unwrap()).collect();

        for account in &accounts {
            let whirlpool : Whirlpool = Whirlpool::from_bytes(&account.data)?;
            println!("{:?}", whirlpool.price());
        }
        // update_accounts.push(accounts);
        // println!("{:?}", chunk);
    }

    // let mut update_accounts = update_accounts
    //     .concat()
    //     .into_iter()
    //     .filter(|s| s.is_some())
    //     .collect::<Vec<Option<Account>>>();
    
    // println!("{:#?}", update_accounts.clone());
    // println!("{:?}", chunked_address);

    // for account in update_accounts {
    //     let whirlpool : Whirlpool = Whirlpool::from_bytes(&account.unwrap().data)?;
    //     println!("{:?}", whirlpool.price());

    // }

    // println!("{:?}", whirlpool);
    Ok(())
    

}
