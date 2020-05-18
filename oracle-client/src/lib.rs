use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use near_sdk::collections::{Map, Set};
// use near_sdk::json_types::U128; // eventually we may use this for expiration
use near_sdk::{AccountId, env, near_bindgen};
use std::collections::HashMap;
use std::str;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct ClientContract {}

#[near_bindgen]
impl ClientContract {
    #[init]
    pub fn new() -> Self {
        // useful snippet to copy/paste, making sure state isn't already initialized
        assert!(env::state_read::<Self>().is_none(), "Already initialized");
        Self {}
    }

    pub fn token_price_callback(&mut self, price: Vec<u8>) -> String {
        let timestamp: u64 = env::block_timestamp();
        let timestamp_as_string = timestamp.to_string();
        let log_timestamp = format!("timestamp: {}", timestamp_as_string);
        env::log(log_timestamp.as_bytes());
        // let price_as_string = str::from_utf8(&price).unwrap();
        let price_as_string = match str::from_utf8(price.as_slice()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence"),
        };
        let log_received = format!("received: {}", price_as_string);
        env::log(log_received.as_bytes());
        return "aloha".to_string();
    }

    pub fn token_price_callback_string(&mut self, price: String) -> String {
        let timestamp: u64 = env::block_timestamp();
        let timestamp_as_string = timestamp.to_string();
        let log_timestamp = format!("timestamp: {}", timestamp_as_string);
        env::log(log_timestamp.as_bytes());
        let log_received = format!("received: {}", price);
        env::log(log_received.as_bytes());
        return "aloha".to_string();
    }

    pub fn stub(&self) -> String { "honua".to_string() }

    pub fn stub_log(&self) { env::log(b"why") }

    pub fn stub_panic(&self) { env::panic(b"forced panic") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    // part of writing unit tests is setting up a mock context
    // this is also a useful list to peek at when wondering what's available in env::*
    fn get_context(input: Vec<u8>, is_view: bool, signer: AccountId) -> VMContext {
        VMContext {
            current_account_id: "alice.testnet".to_string(),
            signer_account_id: signer,
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "jane.testnet".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 19
        }
    }

    // mark individual unit tests with #[test] for them to be registered and fired
    // unlike other frameworks, the function names don't need to be special or have "test" in it
    #[test]
    fn increment() {
        // set up the mock context into the testing environment
        let context = get_context(vec![], false, "robert.testnet".to_string());
        testing_env!(context);
        // instantiate a contract variable with the counter at zero
        let mut contract = ClientContract::new();
        let input: Vec<u8> = vec![1, 9, 9, 1];

        // let input_temp: Vec<u8> = vec![1, 9, 9, 1];
        let input_temp: Vec<u8> = "1991".as_bytes().to_vec();
        let price_as_string = match str::from_utf8(input_temp.as_slice()) {
            Ok(val) => val,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(price_as_string, "1991".to_string());

        let ret = contract.token_price_callback(input);
        println!("Got: {}", ret.to_string());
        assert_eq!("aloha", ret);
    }
}