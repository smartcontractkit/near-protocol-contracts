use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};
use base64::{decode};
use std::str;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub type Base64String = String;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct ClientContract {}

#[near_bindgen]
impl ClientContract {
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