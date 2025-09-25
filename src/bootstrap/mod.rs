use anyhow::Result;
use tokio::fs::create_dir_all;

pub mod orca;
pub mod pool_schema;
pub mod raydium;

pub async fn update_all(data_folder_path: &str, is_test: bool) -> Result<()> {
    create_dir_all(data_folder_path).await?;

    // let orca_bootstrap_task = tokio::spawn(async { orca::fetch_pools(data_folter_path, is_test).await.unwrap() });
    // let raydium_bootstrap_task = tokio::spawn(async { raydium::fetch_pools(data_folter_path, is_test).await.unwrap() });

    let (_, _) = tokio::try_join!(
        orca::fetch_pools(data_folder_path, is_test),
        raydium::fetch_pools(data_folder_path, is_test),
    )?;

    // orca_tokens.extend(raydium_tokens);
    // let all_tokens = orca_tokens;

    // let file = File::create("./cached-blockchain-data/tokens.json").await?;
    // let mut writer = BufWriter::new(file);
    // writer.write_all(b"{\"all_tokens\":").await?;

    // let all_tokens_json = serde_json::to_string(&all_tokens)?;
    // writer.write_all(all_tokens_json.as_bytes()).await?;

    // writer.write_all(b"}").await?;
    // writer.flush().await?;

    Ok(())
}
