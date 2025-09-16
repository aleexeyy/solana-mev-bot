use crate::bootstrap::pool_schema::TokenInfo;



struct Node<'a> {
    token: TokenInfo,
    connections: Vec<&'a TokenInfo>,
}
pub fn build_graph() -> Result<(), Box<dyn std::error::Error>> {

    let pool_files_pathes: [&str; 2] = ["./cached-blockchain-data/orca_pools.json", "./cached-blockchain-data/raydium_pools.json"];

    for pool_path in pool_files_pathes {

    }
    Ok(())
}