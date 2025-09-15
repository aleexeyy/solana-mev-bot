pub mod orca;
pub mod raydium;
pub mod pool_schema;


pub async fn update_all() -> Result<(), Box<dyn std::error::Error>> {
    let orca_bootstrap_task = tokio::spawn(async {
        orca::fetch_pools().await.unwrap();
    });
    let raydium_bootstrap_task = tokio::spawn(async {
        raydium::fetch_pools().await.unwrap();
    });

    let _ = tokio::try_join!(orca_bootstrap_task, raydium_bootstrap_task);

    


    Ok(())
}