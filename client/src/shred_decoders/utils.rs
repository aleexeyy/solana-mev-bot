use std::str::FromStr;

use solana_sdk::pubkey::{Pubkey, PubkeyError, bytes_are_curve_point};

const MAX_SEED_LEN: usize = 32;
const MAX_SEEDS: usize = 16;
const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

pub struct DecodingUtils {}

impl DecodingUtils {
    pub fn find_token_account_address(owner: &Pubkey, token_mint_address: &Pubkey) -> Pubkey {
        let token_program_address =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let program_address =
            Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

        Self::find_token_account_address_and_bump_seed_internal(
            &owner,
            &token_mint_address,
            &token_program_address,
            &program_address,
        )
        .0
    }

    fn find_token_account_address_and_bump_seed_internal(
        owner: &Pubkey,
        token_mint_address: &Pubkey,
        token_program_address: &Pubkey,
        program_address: &Pubkey,
    ) -> (Pubkey, u8) {
        Self::try_find_program_address(
            &[
                &owner.to_bytes(),
                &token_program_address.to_bytes(),
                &token_mint_address.to_bytes(),
            ],
            &program_address,
        )
        .unwrap_or_else(|| core::panic!("Unable to find a viable program address bump seed"))
    }

    fn try_find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Option<(Pubkey, u8)> {
        {
            let mut bump_seed = [u8::MAX];
            for _ in 0..u8::MAX {
                {
                    let mut seeds_with_bump = seeds.to_vec();
                    seeds_with_bump.push(&bump_seed);
                    match Self::create_program_address(&seeds_with_bump, program_id) {
                        Ok(address) => return Some((address, bump_seed[0])),
                        Err(PubkeyError::InvalidSeeds) => (),
                        _ => break,
                    }
                }
                bump_seed[0] -= 1;
            }
            None
        }
    }

    fn create_program_address(
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> anyhow::Result<Pubkey, PubkeyError> {
        if seeds.len() > MAX_SEEDS {
            return Err(PubkeyError::MaxSeedLengthExceeded);
        }
        for seed in seeds.iter() {
            if seed.len() > MAX_SEED_LEN {
                return Err(PubkeyError::MaxSeedLengthExceeded);
            }
        }

        {
            let mut hasher = solana_sha256_hasher::Hasher::default();
            for seed in seeds.iter() {
                hasher.hash(seed);
            }
            hasher.hashv(&[program_id.as_ref(), PDA_MARKER]);
            let hash = hasher.result();

            if bytes_are_curve_point(hash) {
                return Err(PubkeyError::InvalidSeeds);
            }

            Ok(Pubkey::from(hash.to_bytes()))
        }
    }
}
