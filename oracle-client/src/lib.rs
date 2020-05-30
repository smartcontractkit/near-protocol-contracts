use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};
use std::str;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct ClientContract {}

#[near_bindgen]
impl ClientContract {
    // #[init]
    // pub fn new() -> Self {
    //     // useful snippet to copy/paste, making sure state isn't already initialized
    //     assert!(env::state_read::<Self>().is_none(), "Already initialized");
    //     Self {}
    // }

    pub fn token_price_callback(&mut self, price: Vec<u8>) {
        let price_as_string = match str::from_utf8(price.as_slice()) {
            Ok(val) => val,
            Err(_) => env::panic(b"Invalid UTF-8 sequence provided from oracle contract."),
        };
        env::log(format!("Client contract received price: {}", price_as_string).as_bytes());
    }
}