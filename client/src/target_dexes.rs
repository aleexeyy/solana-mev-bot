use std::str::FromStr;

use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Program {
    RaydiumV2,
    RaydiumV3,
    OrcaV3,
    MeteoraV3,
    MeteoraV2,
    Jupiter,
}

impl Program {
    pub fn index(&self) -> usize {
        match self {
            Program::RaydiumV2 => 0,
            Program::RaydiumV3 => 1,
            Program::OrcaV3 => 2,
            Program::MeteoraV3 => 3,
            Program::MeteoraV2 => 4,
            Program::Jupiter => 5,
        }
    }
}

pub static PROGRAM_KEYS: Lazy<[(Program, Pubkey); 6]> = Lazy::new(|| {
    [
        (
            Program::Jupiter,
            Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap(),
        ),
        (
            Program::RaydiumV2,
            Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap(),
        ),
        (
            Program::RaydiumV3,
            Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap(),
        ),
        (
            Program::OrcaV3,
            Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap(),
        ),
        (
            Program::MeteoraV3,
            Pubkey::from_str("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG").unwrap(),
        ),
        (
            Program::MeteoraV2,
            Pubkey::from_str("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB").unwrap(),
        ),
    ]
});

pub fn match_program(key: &Pubkey) -> Option<Program> {
    PROGRAM_KEYS
        .iter()
        .find(|(_, k)| k == key)
        .map(|(prog, _)| *prog)
}
