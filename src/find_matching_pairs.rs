use crate::bootstrap::{orca, pool_schema::{PoolBootstrap, StoredPools, TokenInfo}};
use serde_path_to_error::deserialize;


pub fn get_matching_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let orca_raw_json = std::fs::read_to_string("./cached-blockchain-data/orca_pools.json").expect("Failed to open the orca file");
    let raydium_raw_json = std::fs::read_to_string("./cached-blockchain-data/raydium_pools.json").expect("Failed to open the raydium file");



    let mut orca_deserializer = serde_json::Deserializer::from_str(&orca_raw_json);
    let deserialized_orca_file: StoredPools = deserialize(&mut orca_deserializer).expect("Failed to deserialize Orca File");

    let mut raydium_deserializer = serde_json::Deserializer::from_str(&raydium_raw_json);
    let deserialized_raydium_file: StoredPools = deserialize(&mut raydium_deserializer).expect("Failed to deserialize Raydium File");


    let orca_pools = deserialized_orca_file.all_pools;
    let raydium_pools = deserialized_raydium_file.all_pools;

   let orca_token_pairs: Vec<(TokenInfo, TokenInfo)> = orca_pools
        .iter()
        .map(|pool| {
            if pool.token_a.address > pool.token_b.address {
                (pool.token_b.clone(), pool.token_a.clone())
            } else {
                (pool.token_a.clone(), pool.token_b.clone())
            }
        })
        .collect();

    let raydium_token_pairs: Vec<(TokenInfo, TokenInfo)> = raydium_pools
        .iter()
        .map(|pool| {
            if pool.token_a.address > pool.token_b.address {
                (pool.token_b.clone(), pool.token_a.clone())
            } else {
                (pool.token_a.clone(), pool.token_b.clone())
            }
        })
        .collect();


    for (orca_index, orca_token_pair) in orca_token_pairs.iter().enumerate() {

        for (raydium_index, raydium_token_pair) in raydium_token_pairs.iter().enumerate() {
            if orca_token_pair.0.address == raydium_token_pair.0.address && orca_token_pair.1.address == raydium_token_pair.1.address {
                println!("Orca Pool {:?} \nRaydium Pool: {:?}", orca_pools[orca_index].address.as_ref().unwrap(), raydium_pools[raydium_index].address.as_ref().unwrap());
                println!("--------------------------------------------------------------------")

            }
        }
    }


    // println!("Orca Token Pairs: {:#?}", orca_token_pairs);

    Ok(())
}