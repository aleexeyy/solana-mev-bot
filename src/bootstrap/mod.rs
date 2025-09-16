use tokio::{fs::File, io::{AsyncWriteExt, BufWriter}};

pub mod orca;
pub mod raydium;
pub mod pool_schema;


pub async fn update_all() -> Result<(), Box<dyn std::error::Error>> {
    let orca_bootstrap_task = tokio::spawn(async {
        orca::fetch_pools().await.unwrap()
    });
    let raydium_bootstrap_task = tokio::spawn(async {
        raydium::fetch_pools().await.unwrap()
    });

    let (mut orca_tokens, raydium_tokens) = tokio::try_join!(orca_bootstrap_task, raydium_bootstrap_task)?;

    orca_tokens.extend(raydium_tokens);
    let all_tokens = orca_tokens;

    let file = File::create("./cached-blockchain-data/tokens.json").await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"{\"all_tokens\":").await?;

    let all_tokens_json = serde_json::to_string(&all_tokens)?;
    writer.write_all(all_tokens_json.as_bytes()).await?;

    writer.write_all(b"}").await?;
    writer.flush().await?;

    Ok(())
}