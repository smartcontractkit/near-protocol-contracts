use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use near_sdk::collections::{Map, Set};
use near_sdk::json_types::U128; // eventually we may use this for expiration
use near_sdk::{AccountId, env, near_bindgen, PromiseResult};
use std::collections::HashMap;
use serde_json::json;
use std::str;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const EXPIRY_TIME: u64 = 5 * 60 * 1000_000_000;

// TODO: Adjust based on what makes sense for NEAR
const MINIMUM_CONSUMER_GAS_LIMIT: u64 = 1000_000_000;
const SINGLE_CALL_GAS: u64 = 200000000000000;
// const SINGLE_CALL_GAS: u64 = 10000000000;
/*
200000000000000
10000000000000000
*/

const LINK_TOKEN_ADDRESS: &str = "v0.link.testnet";

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
    /// This is the entry point that will use the escrow transfer_from.
    /// Afterwards, it essentially calls itself (store_request) which stores the request in state.
    pub fn request(&mut self, payment: U128, spec_id: Vec<u8>, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Vec<u8>) {
        self.check_callback_address(&callback_address);

        // first transfer token
        let promise_transfer_tokens = env::promise_create(
            LINK_TOKEN_ADDRESS.to_string(),
            b"transfer_from",
            json!({
                "owner_id": env::predecessor_account_id(),
                "new_owner_id": env::current_account_id(),
                "amount": payment,
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );

        // call this contract's request function after the transfer
        let promise_call_self_request = env::promise_then(
            promise_transfer_tokens,
            env::current_account_id(),
            b"store_request",
            json!({
                "sender": env::predecessor_account_id(),
                "payment": payment,
                "spec_id": spec_id,
                "callback_address": callback_address,
                "callback_method": callback_method,
                "nonce": nonce,
                "data_version": data_version,
                "data": data
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS * 3
        );
        env::log(format!("Single call times three {:?}", SINGLE_CALL_GAS * 3).as_bytes());
        env::log(format!("Prepaid gas {:?}", env::prepaid_gas()).as_bytes());

        env::promise_return(promise_call_self_request);
    }

    pub fn store_request(&mut self, sender: AccountId, payment: U128, spec_id: Vec<u8>, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Vec<u8>) {
        // this method should only ever be called from this contract
        self.only_owner_signed();
        // TODO: fix this "if" workaround until I can figure out how to write tests with promises
        if cfg!(target_arch = "wasm32") {
            assert_eq!(env::promise_results_count(), 1);
            // ensure successful promise, meaning tokens are transferred
            match env::promise_result(0) {
                PromiseResult::Successful(_) => {},
                PromiseResult::Failed => env::panic(b"The promise failed. Likely causes include unavailable fungible token balance and/or allowance."),
                PromiseResult::NotReady => env::panic(b"The promise was not ready."),
            };
        }

        // cast arguments in order to be formatted
        let payment_u128: u128 = payment.into();
        let nonce_u128: u128 = nonce.into();

        let request_id_string: String = format!("{}:{}", sender, nonce_u128);
        let request_id_bytes = env::keccak256(request_id_string.as_bytes());

        let existing_commitment = self.commitments.get(&request_id_bytes);

        if existing_commitment.is_some() {
            env::log(b"inside existing commitment is none");
            // User mistakenly gave same request params, refund
            // These calls will panic, so logic will no longer proceed below.
            let promise_transfer_refund = env::promise_create(
                LINK_TOKEN_ADDRESS.to_string(),
                b"transfer",
                json!({
                "owner_id": env::current_account_id(),
                "new_owner_id": env::signer_account_id(),
                "amount": payment,
            }).to_string().as_bytes(),
                0,
                SINGLE_CALL_GAS,
            );

            // call this contract's panic function after refunding
            let promise_panic = env::promise_then(
                promise_transfer_refund,
                env::current_account_id(),
                b"panic",
                json!({
                "error_message": "Must use a unique ID, composed of sender account and nonce."
            }).to_string().as_bytes(),
                0,
                SINGLE_CALL_GAS
            );

            env::promise_return(promise_panic);
            // env::promise_return(promise_transfer_refund);
            // env::panic(b"There already exists a commitment here for that.");
        } else {
            env::log(b"past existing commitment statement");
            let expiration = env::block_timestamp() + EXPIRY_TIME;
            let commitment = env::keccak256(format!("{}:{}:{}:{}", payment_u128, callback_address, callback_method, expiration).as_bytes());

            // store entire request as well
            // TODO: with websockets/subscriptions we can considering using logging instead of state
            let oracle_request = OracleRequest {
                caller_account: sender,
                request_spec: spec_id,
                callback_address,
                callback_method,
                data // TODO: add payment and see if it serializes
            };
            self.requests.insert(request_id_string.clone(), oracle_request);
            self.commitments.insert(&request_id_bytes, &commitment);
        }
    }

    /// TODO: this function has not been tested and is in-progress
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
        let serialized = serde_json::to_string(&self.requests).unwrap();
        return serialized;
    }

    pub fn get_withdrawable_tokens(&self) -> u128 {
        self.withdrawable_tokens
    }

    pub fn reset(&mut self) {
        self.only_owner();
        self.commitments.clear();
        self.requests.clear();
    }

    /// Can be called after a cross-contract call before enforcing a panic
    pub fn panic(&mut self, error_message: String) {
        self.only_owner_signed();
        env::panic(error_message.as_bytes());
    }

    fn has_available_funds(&mut self, amount: u128) {
        assert!(self.withdrawable_tokens >= amount, "Amount requested is greater than withdrawable balance.");
    }

    fn only_owner(&mut self) {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Only contract owner can call this method.");
    }

    /// This is a helper function with the promises happening.
    /// The predecessor will be this account calling itself after transferring
    /// fungible tokens. Used for functions called via promises where we
    /// do not want end user accounts calling them directly.
    fn only_owner_signed(&mut self) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Only contract owner can sign transactions for this method.");
    }

    fn only_authorized_node(&mut self) {
        assert!(self.authorized_nodes.contains(&env::signer_account_id()) || env::signer_account_id() == env::current_account_id(),
            "Not an authorized node to fulfill requests.");
    }

    fn check_callback_address(&mut self, callback_address: &AccountId) {
        assert!(callback_address != &LINK_TOKEN_ADDRESS, "Cannot callback to LINK.")
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
            signer_account_id: "alice_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "alice_near".to_string(),
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

        let sender = "alice_near".to_string();
        let payment_json: U128 = 51319_u128.into();
        let spec_id = vec![1, 9, 1];
        let nonce = 1_u128;
        let nonce_json: U128 = 1_u128.into();
        let data_version_json: U128 = 131_u128.into();
        let data: Vec<u8> = vec![4, 6, 4, 2, 8, 2];
        contract.store_request( "alice_near".to_string(), payment_json, spec_id, "callback.sender.testnet".to_string(), "my_callback_fn".to_string(), nonce_json, data_version_json, data);

        // second validate the serialized requests
        let serialized_output = contract.get_all_requests();
        assert_eq!("{\"alice_near:1\":{\"caller_account\":\"alice_near\",\"request_spec\":[1,9,1],\"callback_address\":\"callback.sender.testnet\",\"callback_method\":\"my_callback_fn\",\"data\":[4,6,4,2,8,2]}}", serialized_output);
        // first validate the commitment is what we expect
        let request_id = env::keccak256(format!("{}:{}", sender, nonce).as_bytes());

        assert_eq!(1, contract.commitments.len(), "Didn't seem to add the request properly.");

        let commitment_val = match contract.commitments.get(&request_id) {
            Some(v) => v,
            None => Vec::new()
        };
        assert_eq!(vec![196, 143, 50, 195, 145, 131, 130, 121, 214, 15, 31, 43, 180, 227, 159, 56, 173, 32, 244, 231, 106, 251, 78, 93, 84, 24, 213, 92, 81, 229, 217, 80], commitment_val);
    }

    #[test]
    fn make_request() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = Oracle::default();

        let payment: U128 = 6_u128.into();
        let spec_id = vec![1, 9, 1];
        let callback_address = "callback.testnet".to_string();
        let callback_method = "test_callback".to_string();
        let nonce: U128 = 1_u128.into();
        let data_version: U128 = 131_u128.into();
        let data: Vec<u8> = vec![4, 6, 4, 2, 8, 2];

        contract.request(payment, spec_id, callback_address, callback_method, nonce, data_version, data);
        // TODO: figure out why promise isn't going through
    }
}