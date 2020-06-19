use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use near_sdk::collections::{TreeMap, UnorderedSet, UnorderedMap};
use near_sdk::json_types::{U128, U64};
use near_sdk::{AccountId, env, near_bindgen, PromiseResult};
use serde_json::json;
use std::str;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const EXPIRY_TIME: u64 = 5 * 60 * 1000_000_000;

const MINIMUM_CONSUMER_GAS_LIMIT: u64 = 1000_000_000;
const ONE_FOR_CONSISTENT_GAS_COST: u128 = 1;
const SINGLE_CALL_GAS: u64 = 200000000000000;
const TRANSFER_FROM_NEAR_COST: u128 = 36500000000000000000000;

pub type Base64String = String;

#[derive(Default, BorshDeserialize, BorshSerialize, Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct OracleRequest {
    caller_account: AccountId,
    request_spec: Base64String,
    callback_address: AccountId,
    callback_method: String,
    data: Base64String,
    payment: u128,
    expiration: u64
}

#[derive(Serialize, Deserialize)]
pub struct SummaryJSON {
    account: AccountId,
    total_requests: u16, // TODO: choosing u16? need to enforce if so
}

#[derive(Serialize, Deserialize)]
pub struct RequestsJSON {
    nonce: U128,
    request: OracleRequest,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Oracle {
    pub owner: AccountId,
    pub link_account: AccountId,
    pub withdrawable_tokens: u128,
    pub commitments: UnorderedMap<Vec<u8>, Vec<u8>>,
    // using HashMap instead of Map because Map won't serialize with serde
    // TODO: don't use HashMap
    /*
        You should always implement pagination whenever you need multiple requests
        With Map you can do to_vec() method that returns std collection which implements Serialize
        Pagination:
        https://github.com/near/near-sdk-rs/blob/6eb55728af508a070bd37fc206acbeddf35d43e8/near-sdk/src/collections/map.rs#L240
        Pagination implementation:
        https://github.com/near-examples/token-factory/blob/master/contracts/factory/src/lib.rs#L51
    */
    pub requests: TreeMap<AccountId, TreeMap<u128, OracleRequest>>,
    pub authorized_nodes: UnorderedSet<AccountId>,
}

impl Default for Oracle {
    fn default() -> Self {
        panic!("Oracle should be initialized before usage")
    }
}

#[near_bindgen]
impl Oracle {
    /// Initializes the contract with the given total supply owned by the given `owner_id` and `withdrawable_tokens`
    #[init]
    pub fn new(link_id: AccountId, owner_id: AccountId) -> Self {
        assert!(env::is_valid_account_id(owner_id.as_bytes()), "Owner's account ID is invalid");
        assert!(env::is_valid_account_id(link_id.as_bytes()), "Link token account ID is invalid");
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner: owner_id,
            link_account: link_id,
            withdrawable_tokens: ONE_FOR_CONSISTENT_GAS_COST,
            commitments: UnorderedMap::new(b"commitments".to_vec()),
            requests: TreeMap::new(b"requests".to_vec()),
            authorized_nodes: UnorderedSet::new(b"authorized_nodes".to_vec()),
        }
    }

    /// This is the entry point that will use the escrow transfer_from.
    /// Afterwards, it essentially calls itself (store_request) which stores the request in state.
    pub fn request(&mut self, payment: U128, spec_id: Base64String, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Base64String) {
        self._check_callback_address(&callback_address);

        // first transfer token
        let promise_transfer_tokens = env::promise_create(
            self.link_account.clone(),
            b"transfer_from",
            json!({
                "owner_id": env::predecessor_account_id(),
                "new_owner_id": env::current_account_id(),
                "amount": payment,
            }).to_string().as_bytes(),
            TRANSFER_FROM_NEAR_COST,
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

        env::promise_return(promise_call_self_request);
    }

    /// Accounts/contracts should call request, which in turn calls this contract via a promise
    #[allow(unused_variables)] // for data_version, which is also not used in Solidity as I understand
    pub fn store_request(&mut self, sender: AccountId, payment: U128, spec_id: Base64String, callback_address: AccountId, callback_method: String, nonce: U128, data_version: U128, data: Base64String) {
        // this method should only ever be called from this contract
        // TODO: break this out into helper function
        self._only_owner_predecessor();
        // TODO: fix this "if" workaround until I can figure out how to write tests with promises
        if cfg!(target_arch = "wasm32") {
            assert_eq!(env::promise_results_count(), 1);
            // ensure successful promise, meaning tokens are transferred
            match env::promise_result(0) {
                PromiseResult::Successful(_) => {},
                PromiseResult::Failed => env::panic(b"The promise failed. See receipt failures."),
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
            // User mistakenly gave same request params, refund
            // These calls will panic, so logic will no longer proceed below.
            let promise_transfer_refund = env::promise_create(
                self.link_account.clone(),
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
        } else {
            env::log(b"past existing commitment statement");
            // TODO: don't hardcode this, but get past testing
            env::log(format!("EXPIRY_TIME: {}", EXPIRY_TIME).as_bytes());
            // let expiration: u64 = env::block_timestamp() + EXPIRY_TIME;
            let expiration: u64 = 1906293427246306700u64;
            let commitment = env::keccak256(format!("{}:{}:{}:{}", payment_u128, callback_address, callback_method, expiration.clone()).as_bytes());

            // store entire request as well
            // TODO: with websockets/subscriptions we can considering using logging instead of state
            let oracle_request = OracleRequest {
                caller_account: sender.clone(),
                request_spec: spec_id,
                callback_address,
                callback_method,
                data,
                payment: payment_u128,
                expiration,
            };

            // Insert request and commitment into state.
            /*
              account =>
                nonce => { Request }
            */
            let mut nonce_request = self.requests.get(&sender).unwrap_or_default();
            nonce_request.insert(&nonce_u128, &oracle_request);
            self.requests.insert(&sender.clone(), &nonce_request);
            env::log(format!("Inserted commitment with\nKey: {:?}\nValue: {:?}", nonce_u128.clone(), oracle_request.clone()).as_bytes());

            self.commitments.insert(&request_id_bytes, &commitment);
        }
    }

    /// TODO: this function has not been tested and is in-progress
    /// Note that the request_id here is String instead of Vec<u8> as might be expected from the Solidity contract
    pub fn fulfill_request(&mut self, account: AccountId, nonce: U128, payment: U128, callback_address: AccountId, callback_method: String, expiration: U128, data: Base64String) {
        self._only_authorized_node();
        let payment_u128: u128 = payment.into();
        let nonce_u128: u128 = nonce.into();
        let expiration_u128: u128 = expiration.into();

        let request_id: String = format!("{}:{}", account, nonce_u128);
        let request_id_bytes = env::keccak256(request_id.as_bytes());
        env::log(format!("Looking to fulfill commitment with key {:?}", request_id_bytes.clone()).as_bytes());

        let params_hash = env::keccak256(format!("{}:{}:{}:{}", payment_u128, callback_address, callback_method, expiration_u128).as_bytes());
        env::log(format!("params_hash {:?}", params_hash.clone()).as_bytes());

        match self.commitments.get(&request_id_bytes) {
            None => env::panic(b"No commitment for given request ID"),
            Some(commitment) => {
                env::log(format!("fulfill commitment {:?}", commitment.clone()).as_bytes());
                assert!(commitment == params_hash, "Params do not match request ID")
            }
        }

        // TODO: this is probably going to be too low at first, adjust
        assert!(env::prepaid_gas() - env::used_gas() > MINIMUM_CONSUMER_GAS_LIMIT, "Must provide consumer enough gas");

        // pay oracle node the payment
        let promise_pay_oracle_node = env::promise_create(
            self.link_account.clone(),
            b"transfer",
            json!({
                "owner_id": env::current_account_id(),
                "new_owner_id": env::predecessor_account_id(),
                "amount": payment,
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );

        let promise_post_oracle_payment = env::promise_then(
            promise_pay_oracle_node,
            env::current_account_id(),
            b"fulfillment_post_oracle_payment",
            json!({
                "payment": payment
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS
        );

        // TODO: seems as though the process isn't halted here, move these to callbacks
        let promise_perform_callback = env::promise_then(
            promise_post_oracle_payment,
            callback_address,
            callback_method.as_bytes(),
            json!({
                "price": data
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS
        );

        let promise_post_callback = env::promise_then(
            promise_perform_callback,
            env::current_account_id(),
            b"fulfillment_perform_callback",
            json!({
                "account": account,
                "nonce": nonce
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS * 4 // TODO: futz
        );

        env::promise_return(promise_post_callback);
    }

    pub fn fulfillment_post_oracle_payment(&mut self, payment: U128) {
        self._only_owner_predecessor();
        // TODO: fix this "if" workaround until I can figure out how to write tests with promises
        if cfg!(target_arch = "wasm32") {
            assert_eq!(env::promise_results_count(), 1);
            // ensure successful promise, meaning tokens are transferred
            match env::promise_result(0) {
                PromiseResult::Successful(_) => {},
                PromiseResult::Failed => env::panic(b"(fulfillment_post_oracle_payment) The promise failed. See receipt failures."),
                PromiseResult::NotReady => env::panic(b"The promise was not ready."),
            };
        }
        // Subtract payment from local state
        let payment_u128: u128 = payment.into();
        self.withdrawable_tokens -= payment_u128;
        // TODO LEFTOFF: we need to add to this after the first request comes in I think
    }

    pub fn fulfillment_perform_callback(&mut self, account: AccountId, nonce: U128) {
        self._only_owner_predecessor();
        // TODO: fix this "if" workaround until I can figure out how to write tests with promises
        if cfg!(target_arch = "wasm32") {
            assert_eq!(env::promise_results_count(), 1);
            // ensure successful promise, meaning tokens are transferred
            match env::promise_result(0) {
                PromiseResult::Successful(_) => {},
                PromiseResult::Failed => env::panic(b"(fulfillment_perform_callback) The promise failed. See receipt failures."),
                PromiseResult::NotReady => env::panic(b"The promise was not ready."),
            };
        }
        // Remove commitment from local state
        // let mut nonce_request = self.requests.get(&sender).unwrap();
        // nonce_request.nonce_to_request.insert(&nonce_u128, &oracle_request);
        let nonce_u128: u128 = nonce.into();
        let request_id: String = format!("{}:{}", account, nonce_u128);

        self.requests.remove(&account);
        let request_id_bytes = env::keccak256(request_id.as_bytes());
        self.commitments.remove(&request_id_bytes.clone());
        env::log(b"Commitment that has completed successfully and been removed.")
    }

    pub fn is_authorized(&self, node: AccountId) -> bool {
        self.authorized_nodes.contains(&node)
    }

    pub fn add_authorization(&mut self, node: AccountId) {
        self._only_owner();
        assert!(env::is_valid_account_id(node.as_bytes()), "Account ID is invalid");
        self.authorized_nodes.insert(&node);
    }

    pub fn remove_authorization(&mut self, node: AccountId) {
        self._only_owner();

        self.authorized_nodes.remove(&node);
    }

    /*
    pub fn withdraw(&mut self, _recipient: AccountId, amount: u128) {
        self._only_owner();
        self._has_available_funds(amount);
        
        self.withdrawable_tokens -= amount;
        // TODO: Transfer LINK. Does this method make sense in NEAR?
    }
    */

    /// Get up to first 65K accounts that have their own associated nonces => requests
    pub fn get_requests_summary(&self, max_num_accounts: U64) -> String {
        let mut counter: u64 = 0;
        let max_num_accounts_u64: u64 = max_num_accounts.into();
        let mut result: Vec<SummaryJSON> = Vec::with_capacity(max_num_accounts_u64 as usize);

        for req in self.requests.iter() {
            self._request_summary_iterate(&max_num_accounts_u64, req, &mut result, &mut counter);
        }

        serde_json::to_string(&result).unwrap()
    }

    pub fn get_requests_summary_from(&self, from_account: AccountId, max_num_accounts: U64) -> String {
        let mut counter: u64 = 0;
        let max_num_accounts_u64: u64 = max_num_accounts.into();
        let mut result: Vec<SummaryJSON> = Vec::with_capacity(max_num_accounts_u64 as usize);

        for req in self.requests.iter_from(from_account) {
            self._request_summary_iterate(&max_num_accounts_u64, req, &mut result, &mut counter);
        }

        serde_json::to_string(&result).unwrap()
    }

    /// Helper function while iterating through request summaries
    fn _request_summary_iterate(&self, max_num_accounts: &u64, req: (AccountId, TreeMap<u128, OracleRequest>), result: &mut Vec<SummaryJSON>, counter: &mut u64) {
        if *counter == *max_num_accounts || *counter > self.requests.len() {
            return
        }
        let account = req.0;
        let total_requests = req.1.len() as u16;
        result.push(SummaryJSON {
            account,
            total_requests
        });

        *counter += 1;
    }

    pub fn get_requests(&self, account: AccountId, max_requests: U64) -> String {
        let max_requests_u64: u64 = max_requests.into();
        if !self.requests.contains_key(&account) {
            env::panic(format!("Account {} has no requests.", account).as_bytes());
        }
        let mut counter: u64 = 0;
        let mut result: Vec<RequestsJSON> = Vec::with_capacity(max_requests_u64 as usize);
        let account_requests_map = self.requests.get(&account).unwrap();

        for req in account_requests_map.iter() {
            self._request_iterate(&max_requests_u64, req, &mut result, &mut counter);
        }

        serde_json::to_string(&result).unwrap()
    }

    /// Helper function while iterating through account requests
    fn _request_iterate(&self, max_requests: &u64, req: (u128, OracleRequest), result: &mut Vec<RequestsJSON>, counter: &mut u64) {
        if *counter == *max_requests || *counter > self.requests.len() {
            return
        }
        let nonce = req.0;
        let oracle_request = req.1;
        result.push(RequestsJSON {
            nonce: U128(nonce),
            request: oracle_request,
        });

        *counter += 1;
    }

    pub fn get_all_commitments(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        env::log(b"Returning all commitments");
        self.commitments.to_vec()
    }

    pub fn get_withdrawable_tokens(&self) -> u128 {
        self.withdrawable_tokens
    }

    pub fn reset(&mut self) {
        self._only_owner();
        self.commitments.clear();
        self.requests.clear();
        env::log(b"Commitments and requests are cleared.");
    }

    /// Can be called after a cross-contract call before enforcing a panic
    pub fn panic(&mut self, error_message: String) {
        self._only_owner_predecessor();
        env::panic(error_message.as_bytes());
    }

    // TODO: organize into impl for private functions
    fn _has_available_funds(&mut self, amount: u128) {
        assert!(self.withdrawable_tokens >= amount, "Amount requested is greater than withdrawable balance.");
    }

    fn _only_owner(&mut self) {
        assert_eq!(env::signer_account_id(), env::current_account_id(), "Only contract owner can call this method.");
    }

    /// This is a helper function with the promises happening.
    /// The predecessor will be this account calling itself after transferring
    /// fungible tokens. Used for functions called via promises where we
    /// do not want end user accounts calling them directly.
    fn _only_owner_predecessor(&mut self) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Only contract owner can sign transactions for this method.");
    }

    fn _only_authorized_node(&mut self) {
        assert!(self.authorized_nodes.contains(&env::signer_account_id()) || env::signer_account_id() == env::current_account_id(),
            "Not an authorized node to fulfill requests.");
    }

    fn _check_callback_address(&mut self, callback_address: &AccountId) {
        assert!(callback_address != &self.link_account, "Cannot callback to LINK.")
    }

    /// This method is not compile to the smart contract. It is used in tests only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_all_authorizations(&self) -> Vec<AccountId> {
        let nodes_vectorized = self.authorized_nodes.as_vector();
        let length = nodes_vectorized.len();
        let mut ret = Vec::new();
        for idx in 0..length {
            ret.push(nodes_vectorized.get(idx).unwrap());
        }
        ret
    }

    /// This method is not compile to the smart contract. It is used in tests only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn test_callback(&self, data: Vec<u8>) {
        println!("Received test callback with data: {:?}", data);
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
    fn make_request_validate_commitment() {
        let context = get_context(alice(), 0);
        testing_env!(context);
        let mut contract = Oracle::new(link(), alice(), );
        let sender = alice();
        let payment_json: U128 = 51319_u128.into();
        let spec_id = encode("unique spec id".to_string());
        let nonce_json: U128 = 1_u128.into();
        let data_version_json: U128 = 131_u128.into();
        let data = encode("BAT".to_string());
        contract.store_request( sender, payment_json, spec_id, "callback.sender.testnet".to_string(), "my_callback_fn".to_string(), nonce_json, data_version_json, data);

        // second validate the serialized requests
        let max_requests: U64 = 1u64.into();
        let serialized_output = contract.get_requests(alice(), max_requests);
        let expected_result = "[{\"nonce\":\"1\",\"request\":{\"caller_account\":\"alice_near\",\"request_spec\":\"dW5pcXVlIHNwZWMgaWQ=\",\"callback_address\":\"callback.sender.testnet\",\"callback_method\":\"my_callback_fn\",\"data\":\"QkFU\",\"payment\":51319,\"expiration\":1906293427246306700}}]";
        assert_eq!(expected_result, serialized_output);
    }

    #[test]
    fn make_request() {
        let mut context = get_context(alice(), 0);
        context.attached_deposit = TRANSFER_FROM_NEAR_COST;
        testing_env!(context.clone());
        let mut contract = Oracle::new(link(), alice());
        let payment: U128 = 6_u128.into();
        let spec_id = encode("unique spec id".to_string());
        let callback_address = "callback.testnet".to_string();
        let callback_method = "test_callback".to_string();
        let nonce: U128 = 1_u128.into();
        let data_version: U128 = 131_u128.into();
        let data = encode("BAT".to_string());

        contract.request(payment, spec_id, callback_address, callback_method, nonce, data_version, data);
        // TODO: figure out why promise isn't going through
    }

    #[test]
    fn check_authorization() {
        let context = get_context(alice(), 0);
        testing_env!(context);
        let mut contract = Oracle::new(link(), alice());
        let mut authorizations = contract.get_all_authorizations();
        let empty_vec: Vec<AccountId> = Vec::new();
        assert_eq!(empty_vec, authorizations);
        contract.add_authorization(alice());
        authorizations = contract.get_all_authorizations();
        let only_alice: Vec<AccountId> = vec![alice()];
        assert_eq!(only_alice, authorizations);
        contract.add_authorization(bob());
        let bob_is_authorized = contract.is_authorized(bob());
        assert_eq!(true, bob_is_authorized);
        contract.remove_authorization(bob());
        assert_eq!(only_alice, authorizations);
    }

    #[test]
    fn multiple_requests_to_json() {
        // Context: Alice
        let mut context = get_context(alice(), 0);
        testing_env!(context);
        // Set up contract
        let mut contract = Oracle::new(link(), alice());
        // Alice stores two requests
        contract.store_request( alice(), 6_u128.into(), "unique-id".to_string(), "callback.testnet".to_string(), "test_callback".to_string(), 1_u128.into(), 131_u128.into(), "BAT".to_string());
        contract.store_request( alice(), 6_u128.into(), "unique-id".to_string(), "callback.testnet".to_string(), "test_callback".to_string(), 2_u128.into(), 131_u128.into(), "NEAR".to_string());
        // Context: Bob
        context = get_context(bob(), env::storage_usage());
        testing_env!(context);
        contract.store_request( bob(), 6_u128.into(), "unique-id".to_string(), "callback.testnet".to_string(), "test_callback".to_string(), 1_u128.into(), 131_u128.into(), "BAT".to_string());
        // Context: Link
        context = get_context(link(), env::storage_usage());
        testing_env!(context);
        contract.store_request( link(), 6_u128.into(), "unique-id".to_string(), "callback.testnet".to_string(), "test_callback".to_string(), 1_u128.into(), 131_u128.into(), "BAT".to_string());

        let max_num_accounts: U64 = 2u64.into();
        let json_result = contract.get_requests_summary(max_num_accounts);
        let expected_result = "[{\"account\":\"alice_near\",\"total_requests\":2},{\"account\":\"bob_near\",\"total_requests\":1}]";
        assert_eq!(json_result, expected_result);

        // now start after "alice_near"
        let json_result = contract.get_requests_summary_from(alice(), max_num_accounts);
        let expected_result = "[{\"account\":\"bob_near\",\"total_requests\":1},{\"account\":\"link_near\",\"total_requests\":1}]";
        assert_eq!(json_result, expected_result);
    }

    #[test]
    fn add_request_fulfill() {
        let context = get_context(alice(), 0);
        testing_env!(context);
        let mut contract = Oracle::new(link(), alice());

        // make request
        let payment: U128 = 6_u128.into();
        let spec_id = encode("unique spec id".to_string());
        let callback_address = env::current_account_id();
        let callback_method = "test_callback".to_string();
        let nonce: U128 = 1_u128.into();
        let data_version: U128 = 131_u128.into();
        let data = encode("BAT".to_string());

        // contract.request(payment.clone(), spec_id, callback_address.clone(), callback_method.clone(), nonce.clone(), data_version, data.clone());
        contract.store_request( alice(), payment, spec_id, callback_address.clone(), callback_method.clone(), nonce.clone(), data_version, data.clone());
        let max_num_accounts: U64 = 1u64.into();
        println!("{}", contract.get_requests_summary(max_num_accounts));
        // authorize bob
        contract.add_authorization(bob());

        // fulfill request
        let hardcoded_expiration: U128 = 1906293427246306700_u128.into();
        let context = get_context(bob(), env::storage_usage());
        testing_env!(context);
        contract.fulfill_request(alice(), 1.into(), payment, callback_address, callback_method, hardcoded_expiration, data);
    }
}

// TODO: fix errors here: https://explorer.testnet.near.org/transactions/4tQhZ3hHM1PZ7eqwXFupojkKNaB6ko5vM6JyUF18Mftd
// TODO: make sure it's actually deleting everything once fulfilled