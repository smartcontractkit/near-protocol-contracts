use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, near_bindgen, AccountId};
use base64::{decode};
use std::str;
use near_sdk::json_types::U128;

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
}

impl Default for ClientContract {
    fn default() -> Self {
        panic!("Oracle client should be initialized before usage")
    }
}

#[near_bindgen]
impl ClientContract {
    #[init]
    pub fn new(oracle_account: AccountId) -> Self {
        Self {
            oracle_account,
            nonce: 0,
        }
    }

    /*
fn request(&mut self, payment: U128, spec_id: Base64String, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Base64String);
     */

    /// symbol: Base64-encoded token symbol
    pub fn demo_token_price(&mut self, symbol: String) {
        // For the sake of demo, a few hardcoded values
        let payment = U128(10);
        let spec_id: Base64String = "dW5pcXVlIHNwZWMgaWQ=".to_string();
        let nonce: U128 = self.nonce.into();
        self.nonce += 1;

        ext_oracle::request(payment, spec_id, env::current_account_id(), "token_price_callback".to_string(), nonce, U128(1), symbol, &self.oracle_account, 0, SINGLE_CALL_GAS);
    }
    
    #[allow(dead_code)] // This function gets called from the oracle
    pub fn token_price_callback(&mut self, price: Base64String) {
        let base64_price = match str::from_utf8(price.as_bytes()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence provided from oracle contract."),
        };
        let decoded_price_vec = decode(base64_price).unwrap();
        let price_readable = match str::from_utf8(decoded_price_vec.as_slice()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence in Base64 decoded value."),
        };
        env::log(format!("Client contract received price: {:?}", price_readable).as_bytes());
    }
}