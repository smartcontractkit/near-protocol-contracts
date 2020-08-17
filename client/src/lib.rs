use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, near_bindgen, AccountId};
use near_sdk::collections::TreeMap;
use base64::{decode};
use std::str;
use near_sdk::json_types::U128;
use std::collections::HashMap;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
const SINGLE_CALL_GAS: u64 = 200_000_000_000_000;

pub type Base64String = String;

#[ext_contract(ext_oracle)]
pub trait ExtOracleContract {
    fn request(&mut self, payment: U128, spec_id: Base64String, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Base64String);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
struct ClientContract {
    // Note: for this simple demo we'll store the oracle node in state like this
    // There's no reason why client contracts can't call various oracle contracts.
    oracle_account: AccountId,
    nonce: u128,
    received: TreeMap<u128, String>,
}

impl Default for ClientContract {
    fn default() -> Self {
        panic!("Oracle client should be initialized before usage")
    }
}

#[near_bindgen]
impl ClientContract {
    #[allow(dead_code)]
    #[init]
    pub fn new(oracle_account: AccountId) -> Self {
        Self {
            oracle_account,
            nonce: 0_u128,
            received: TreeMap::new(b"r".to_vec()),
        }
    }

    /// symbol: Base64-encoded token symbol
    #[allow(dead_code)] // This function gets called from the oracle
    pub fn get_token_price(&mut self, symbol: String, spec_id: Base64String) -> U128 {
        // For the sake of demo, a few hardcoded values
        let payment = U128(10);
        self.nonce += 1;
        let nonce: U128 = self.nonce.into();

        ext_oracle::request(payment, spec_id, env::current_account_id(), "token_price_callback".to_string(), nonce, U128(1), symbol, &self.oracle_account, 0, SINGLE_CALL_GAS);
        U128(self.nonce)
    }

    #[allow(dead_code)] // This function gets called from the oracle
    pub fn token_price_callback(&mut self, nonce: U128, answer: Base64String) {
        let base64_price = match str::from_utf8(answer.as_bytes()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence provided from oracle contract."),
        };
        let decoded_price_vec = decode(base64_price).unwrap();
        let price_readable = match str::from_utf8(decoded_price_vec.as_slice()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence in Base64 decoded value."),
        };
        env::log(format!("Client contract received price: {:?}", price_readable).as_bytes());
        self.received.insert(&nonce.0, &price_readable.to_string());
    }

    // using String instead of U128 because
    // the trait `std::cmp::Eq` is not implemented for `near_sdk::json_types::integers::U128`
    #[allow(dead_code)]
    pub fn get_received_vals(&self, max: U128) -> HashMap<String, String> {
        let mut counter: u128 = 0;
        let mut result: HashMap<String, String> = HashMap::new();
        for answer in self.received.iter() {
            if counter == max.0 || counter > self.received.len() as u128 {
                break;
            }
            result.insert(answer.0.to_string(), answer.1);
            counter += 1;
        }
        result
    }

    #[allow(dead_code)]
    pub fn get_received_val(&self, nonce: U128) -> String {
        let nonce_u128: u128 = nonce.into();
        self.received.get(&nonce_u128).unwrap_or("-1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{MockedBlockchain, StorageUsage};
    use near_sdk::{testing_env, VMContext};
    use base64::{encode};

    fn link() -> AccountId { "link_near".to_string() }

    fn alice() -> AccountId { "alice_near".to_string() }

    fn bob() -> AccountId { "bob_near".to_string() }
    fn oracle() -> AccountId { "oracle.testnet".to_string() }

    fn get_context(signer_account_id: AccountId, storage_usage: StorageUsage) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id,
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: alice(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            epoch_height: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn test_token_price() {
        let context = get_context(alice(), 0);
        testing_env!(context);
        let mut contract = ClientContract::new(oracle() );
        let mut returned_nonce = contract.get_token_price("eyJnZXQiOiJodHRwczovL21pbi1hcGkuY3J5cHRvY29tcGFyZS5jb20vZGF0YS9wcmljZT9mc3ltPUVUSCZ0c3ltcz1VU0QiLCJwYXRoIjoiVVNEIiwidGltZXMiOjEwMH0".to_string(), "dW5pcXVlIHNwZWMgaWQ=".to_string());
        assert_eq!(U128(1), returned_nonce);
        returned_nonce = contract.get_token_price("eyJnZXQiOiJodHRwczovL21pbi1hcGkuY3J5cHRvY29tcGFyZS5jb20vZGF0YS9wcmljZT9mc3ltPUVUSCZ0c3ltcz1VU0QiLCJwYXRoIjoiVVNEIiwidGltZXMiOjEwMH0".to_string(), "dW5pcXVlIHNwZWMgaWQ=".to_string());
        assert_eq!(U128(2), returned_nonce);
    }
}
