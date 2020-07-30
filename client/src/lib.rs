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
            nonce: 0,
            received: TreeMap::new(b"r".to_vec()),
        }
    }

    /// symbol: Base64-encoded token symbol
    #[allow(dead_code)] // This function gets called from the oracle
    pub fn demo_token_price(&mut self, symbol: String, spec_id: Base64String) {
        // For the sake of demo, a few hardcoded values
        let payment = U128(10);
        let nonce: U128 = self.nonce.into();
        self.nonce += 1;

        ext_oracle::request(payment, spec_id, env::current_account_id(), "token_price_callback".to_string(), nonce, U128(1), symbol, &self.oracle_account, 0, SINGLE_CALL_GAS);
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
}