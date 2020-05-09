use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use near_sdk::collections::{Map, Set};
// use near_sdk::json_types::U128; // eventually we may use this for expiration
use near_sdk::{AccountId, env, near_bindgen};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const EXPIRY_TIME: u64 = 5 * 60 * 1000_000_000;

// TODO: Adjust based on what makes sense for NEAR
const MINIMUM_CONSUMER_GAS_LIMIT: u64 = 1000_000_000;

const LINK_TOKEN_ADDRESS: &str = "link.testnet";

#[derive(Default, BorshDeserialize, BorshSerialize)]
#[derive(Serialize, Deserialize)]
pub struct OracleRequest {
    caller_account: AccountId,
    request_spec: Vec<u8>,
    callback_address: AccountId,
    callback_method: String,
    data: Vec<u8>
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Oracle {
    pub withdrawable_tokens: u128,
    pub commitments: Map<Vec<u8>, Vec<u8>>,
    // using HashMap instead of Map because Map won't serialize with serde
    pub requests: HashMap<String, OracleRequest>,
    pub authorized_nodes: Set<AccountId>,
}

impl Default for Oracle {
    fn default() -> Self {
        Self {
            withdrawable_tokens: 0,
            commitments: Map::new(b"commitments".to_vec()),
            requests: HashMap::new(),
            authorized_nodes: Set::new(b"authorized_nodes".to_vec()),
        }
    }
}

#[near_bindgen]
impl Oracle {
    pub fn request(&mut self, sender: AccountId, payment: u128, _spec_id: Vec<u8>, callback_address: AccountId, callback_method: String, nonce: u128, _data_version: u128, _data: Vec<u8>) {
        // TODO: I assume onlyLINK not needed as this won't be called by token anymore
        // TODO: Some other way to make sure there is payment should be used
        self.check_callback_address(&callback_address);

        let request_id_string: String = format!("{}:{}", sender, nonce);
        let request_id_bytes = env::keccak256(request_id_string.as_bytes());

        let existing_commitment = self.commitments.get(&request_id_bytes);
        assert!(existing_commitment.is_none(), "Must use a unique ID");

        let expiration = env::block_timestamp() + EXPIRY_TIME;
        let commitment = env::keccak256(format!("{}:{}:{}:{}", payment, callback_address, callback_method, expiration).as_bytes());
        // TODO: Store whole request instead? I assume it's needed for actual execution and we don't have separate event storage.
        let oracle_request = OracleRequest {
            caller_account: sender,
            request_spec: _spec_id,
            callback_address,
            callback_method,
            data: _data
        };
        self.requests.insert(request_id_string.clone(), oracle_request);
        self.commitments.insert(&request_id_bytes, &commitment);
    }

    pub fn fulfill_request(&mut self, request_id: Vec<u8>, payment: u128, callback_address: AccountId, callback_method: String, expiration: u128, data: Vec<u8>) {
        self.only_authorized_node();

        let params_hash = env::keccak256(format!("{}:{}:{}:{}", payment, callback_address, callback_method, expiration).as_bytes());
        match self.commitments.get(&request_id) {
            None => env::panic(b"No commitment for given request ID"),
            Some(commitment) => assert!(commitment == params_hash, "Params do not match request ID")
        }

        self.withdrawable_tokens += payment;
        self.commitments.remove(&request_id);

        assert!(env::prepaid_gas() - env::used_gas() > MINIMUM_CONSUMER_GAS_LIMIT, "Must provide consumer enough gas");
        // TODO: how much gas to pass?
        // TODO: MIKE - https://github.com/near/near-sdk-rs/blob/master/examples/cross-contract-low-level/src/lib.rs#L99-L105
        env::promise_create(callback_address, callback_method.as_bytes(), &data, 0, MINIMUM_CONSUMER_GAS_LIMIT);

        // TODO: Should this allow caller to wait for promise result?
    }

    pub fn is_authorized(&self, node: AccountId) -> bool {
        self.authorized_nodes.contains(&node)
    }

    pub fn add_authorization(&mut self, node: AccountId) {
        self.only_owner();

        self.authorized_nodes.insert(&node);
    }

    pub fn remove_authorization(&mut self, node: AccountId) {
        self.only_owner();

        self.authorized_nodes.remove(&node);
    }

    pub fn withdraw(&mut self, _recipient: AccountId, amount: u128) {
        self.only_owner();
        self.has_available_funds(amount);
        
        self.withdrawable_tokens -= amount;
        // TODO: Transfer LINK. Does this method make sense in NEAR?
    }

    pub fn get_all_requests(&self) -> String {
        env::log(b"Returning all requests");
        // let keys = self.requests.first().key();
        // return keys;
        let serialized = serde_json::to_string(&self.requests).unwrap();
        return serialized;
    }

    pub fn get_withdrawable_tokens(&self) -> u128 {
        self.withdrawable_tokens
    }

    fn has_available_funds(&mut self, amount: u128) {
        assert!(self.withdrawable_tokens >= amount, "Amount requested is greater than withdrawable balance");
    }

    fn only_owner(&mut self) {
        assert!(env::signer_account_id() == env::current_account_id(), "Only contract owner can call this method");
    }

    fn only_authorized_node(&mut self) {
        assert!(self.authorized_nodes.contains(&env::signer_account_id()) || env::signer_account_id() == env::current_account_id(),
            "Not an authorized node to fulfill requests");
    }

    fn check_callback_address(&mut self, callback_address: &AccountId) {
        assert!(callback_address != &LINK_TOKEN_ADDRESS, "Cannot callback to LINK")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            epoch_height: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn make_request_validate_commitment() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = Oracle::default();

        let sender = "sender.testnet".to_string();
        let payment: u128 = 51319;
        let spec_id = vec![1, 9, 1];
        let nonce: u128 = 1;
        let data_version: u128 = 131;
        let data: Vec<u8> = vec![4, 6, 4, 2, 8, 2];
        contract.request(sender.clone(), payment, spec_id, "callback.sender.testnet".to_string(), "my_callback_fn".to_string(), nonce, data_version, data);

        // first validate the commitment is what we expect
        let request_id = env::keccak256(format!("{}:{}", sender, nonce).as_bytes());
        let commitment_val = match contract.commitments.get(&request_id) {
            Some(v) => v,
            None => Vec::new()
        };
        assert_eq!(vec![196, 143, 50, 195, 145, 131, 130, 121, 214, 15, 31, 43, 180, 227, 159, 56, 173, 32, 244, 231, 106, 251, 78, 93, 84, 24, 213, 92, 81, 229, 217, 80], commitment_val);

        // second validate the serialized requests
        let serialized_output = contract.get_all_requests();
        assert_eq!("{\"sender.testnet:1\":{\"caller_account\":\"sender.testnet\",\"request_spec\":[1,9,1],\"callback_address\":\"callback.sender.testnet\",\"callback_method\":\"my_callback_fn\",\"data\":[4,6,4,2,8,2]}}", serialized_output);
    }
}