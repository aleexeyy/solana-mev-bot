use std::{str::FromStr, sync::Arc};

use anyhow::anyhow;
use solana_sdk::pubkey::{Pubkey, PubkeyError, bytes_are_curve_point};

use crate::shred_decoders::{get_lookup_table_addresses, interfaces::ReducedLookupTable};

const MAX_SEED_LEN: usize = 32;
const MAX_SEEDS: usize = 16;
const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

pub struct DecodingUtils {}

impl DecodingUtils {
    fn resolve_lookup_table_address(
        mut address_index: usize,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
        get_table_addresses: impl Fn(&Pubkey) -> Option<Arc<[Pubkey]>>,
    ) -> anyhow::Result<Pubkey> {
        // Solana v0 dynamic account ordering:
        // 1) For each lookup table in order: all writable indexes
        // 2) For each lookup table in order: all readonly indexes
        for lookup_table in lookup_tables.iter() {
            if address_index < lookup_table.writable_indexes.len() {
                let idx = usize::from(lookup_table.writable_indexes[address_index]);
                let lookup_addresses =
                    get_table_addresses(&lookup_table.account_key).ok_or_else(|| {
                        anyhow!("Unsupported lookup table: {}", lookup_table.account_key)
                    })?;
                return Ok(lookup_addresses[idx]);
            }
            address_index -= lookup_table.writable_indexes.len();
        }

        for lookup_table in lookup_tables.iter() {
            if address_index < lookup_table.readonly_indexes.len() {
                let idx = usize::from(lookup_table.readonly_indexes[address_index]);
                let lookup_addresses =
                    get_table_addresses(&lookup_table.account_key).ok_or_else(|| {
                        anyhow!("Unsupported lookup table: {}", lookup_table.account_key)
                    })?;
                return Ok(lookup_addresses[idx]);
            }
            address_index -= lookup_table.readonly_indexes.len();
        }

        Err(anyhow!("Index out of bounds"))
    }

    pub fn get_account_address(
        index: usize,
        account_keys: &[Pubkey],
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> anyhow::Result<Pubkey> {
        if index < account_keys.len() {
            return Ok(account_keys[index]);
        }

        let address_index = index - account_keys.len();
        Self::resolve_lookup_table_address(address_index, lookup_tables, |table_key| {
            get_lookup_table_addresses(table_key).cloned()
        })
    }

    pub fn find_token_account_address(owner: &Pubkey, token_mint_address: &Pubkey) -> Pubkey {
        let token_program_address =
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
        let program_address =
            Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

        Self::find_token_account_address_and_bump_seed_internal(
            owner,
            token_mint_address,
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
            program_address,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn resolves_v0_loaded_addresses_in_writable_then_readonly_order() {
        let t1 = Pubkey::new_unique();
        let t2 = Pubkey::new_unique();

        let addr_t1: Arc<[Pubkey]> =
            Arc::from(vec![Pubkey::new_unique(), Pubkey::new_unique()].into_boxed_slice());
        let addr_t2: Arc<[Pubkey]> =
            Arc::from(vec![Pubkey::new_unique(), Pubkey::new_unique()].into_boxed_slice());

        let mut tables: HashMap<Pubkey, Arc<[Pubkey]>> = HashMap::new();
        tables.insert(t1, addr_t1.clone());
        tables.insert(t2, addr_t2.clone());

        // For v0 ordering, dynamic keys are:
        // writable: t1[w0], t2[w0], readonly: t1[r0], t2[r0]
        let lookup_tables: Arc<Vec<ReducedLookupTable>> = Arc::new(vec![
            ReducedLookupTable {
                account_key: t1,
                writable_indexes: Arc::from([0u8].as_slice()),
                readonly_indexes: Arc::from([1u8].as_slice()),
            },
            ReducedLookupTable {
                account_key: t2,
                writable_indexes: Arc::from([1u8].as_slice()),
                readonly_indexes: Arc::from([0u8].as_slice()),
            },
        ]);

        let resolved0 = DecodingUtils::resolve_lookup_table_address(0, &lookup_tables, |k| {
            tables.get(k).cloned()
        })
        .unwrap();
        assert_eq!(resolved0, addr_t1[0]);

        let resolved1 = DecodingUtils::resolve_lookup_table_address(1, &lookup_tables, |k| {
            tables.get(k).cloned()
        })
        .unwrap();
        assert_eq!(resolved1, addr_t2[1]);

        let resolved2 = DecodingUtils::resolve_lookup_table_address(2, &lookup_tables, |k| {
            tables.get(k).cloned()
        })
        .unwrap();
        assert_eq!(resolved2, addr_t1[1]);

        let resolved3 = DecodingUtils::resolve_lookup_table_address(3, &lookup_tables, |k| {
            tables.get(k).cloned()
        })
        .unwrap();
        assert_eq!(resolved3, addr_t2[0]);
    }
}
