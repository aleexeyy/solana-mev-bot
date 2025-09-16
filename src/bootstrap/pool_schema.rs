use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TokenInfo {
    pub address: Option<String>,
    pub decimals: Option<u8>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolBootstrap {
    pub address: Option<String>,
    pub fee_rate: Option<u32>,
    pub pool_type: Option<String>,
    pub dex: Option<String>,
    pub tick_spacing: Option<u64>,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    pub token_vault_a: Option<String>,
    pub token_vault_b: Option<String>,
    pub config: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredPools {
    pub all_pools: Vec<PoolBootstrap>
}


impl PoolBootstrap {
    pub fn check(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.address.is_none()       { return Err("Missing Address".into()); }
        if self.pool_type.is_none()     { return Err("Missing PoolType".into()); }
        if self.token_vault_a.is_none() { return Err("Missing Token Vault A".into()); }
        if self.token_vault_b.is_none() { return Err("Missing Token Vault B".into()); }
        if self.fee_rate.is_none()      { return Err("Missing Fee Rate".into()); }
        if self.tick_spacing.is_none()  { return Err("Missing Tick Spacing".into()); }
        if self.config.is_none()  { return Err("Missing Config".into()); }

        if self.token_a.address.is_none() { return Err("Missing Token A Address".into()); }
        if self.token_a.decimals.is_none(){ return Err("Missing Token A Decimals".into()); }
        if self.token_a.name.is_none()    { return Err("Missing Token A Name".into()); }
        if self.token_a.symbol.is_none()  { return Err("Missing Token A Symbol".into()); }

        if self.token_b.address.is_none() { return Err("Missing Token B Address".into()); }
        if self.token_b.decimals.is_none(){ return Err("Missing Token B Decimals".into()); }
        if self.token_b.name.is_none()    { return Err("Missing Token B Name".into()); }
        if self.token_b.symbol.is_none()  { return Err("Missing Token B Symbol".into()); }

        Ok(())
    }
}