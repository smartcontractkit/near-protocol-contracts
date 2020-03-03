use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::collections::{Map, Set};
use near_bindgen::{env, near_bindgen};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const EXPIRY_TIME: u64 = 5 * 60 * 1000_000_000;

// TODO: Adjust based on what makes sense for NEAR
const MINIMUM_CONSUMER_GAS_LIMIT: u64 = 1000_000_000;

const LINK_TOKEN_ADDRESS: &str = "link";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Oracle {
    pub withdrawable_tokens: u128,
    pub commitments: Map<Vec<u8>, Vec<u8>>,
    pub authorized_nodes: Set<String>,
}

impl Default for Oracle {
    fn default() -> Self {
        Self {
            withdrawable_tokens: 0,
            commitments: Map::new(b"commitments".to_vec()),
            authorized_nodes: Set::new(b"authorized_nodes".to_vec()),
        }
    }
}

#[near_bindgen]
impl Oracle {
    pub fn request(&mut self, sender: String, payment: u128, spec_id: Vec<u8>, callback_address: String, callback_method: String, nonce: u128, data_version: u128, data: Vec<u8>) {
        // TODO: I assume onlyLINK not needed as this won't be called by token anymore
        // TODO: Some other way to make sure there is payment should be used
        self.check_callback_address(&callback_address);

        let request_id = env::keccak256(format!("{}:{}", sender, nonce).as_bytes());

        let existing_commitment = self.commitments.get(&request_id);
        assert!(existing_commitment.is_none(), "Must use a unique ID");

        let expiration = env::block_timestamp() + EXPIRY_TIME;
        let commitment = env::keccak256(format!("{}:{}:{}:{}", payment, callback_address, callback_method, expiration).as_bytes());
        // TODO: Store whole request instead? I assume it's needed for actual execution and we don't have separate event storage.
        self.commitments.insert(&request_id, &commitment);
    }

    pub fn fulfill_request(&mut self, request_id: Vec<u8>, payment: u128, callback_address: String, callback_method: String, expiration: u128, data: Vec<u8>) {
        self.only_authorized_node();

        let params_hash = env::keccak256(format!("{}:{}:{}:{}", payment, callback_address, callback_method, expiration).as_bytes());
        match self.commitments.get(&request_id) {
            None => panic!("No commitment for given request ID"),
            Some(commitment) => assert!(commitment == params_hash, "Params do not match request ID")
        }

        self.withdrawable_tokens += payment;
        self.commitments.remove(&request_id);

        assert!(env::prepaid_gas() - env::used_gas() > MINIMUM_CONSUMER_GAS_LIMIT, "Must provide consumer enough gas");
        // TODO: how much gas to pass?
        env::promise_create(callback_address, callback_method.as_bytes(), &data, 0, MINIMUM_CONSUMER_GAS_LIMIT);

        // TODO: Should this allow caller to wait for promise result?
    }

    pub fn is_authorized(&self, node: String) -> bool {
        self.authorized_nodes.contains(&node)
    }

    pub fn add_authorization(&mut self, node: String) {
        self.only_owner();

        self.authorized_nodes.insert(&node);
    }

    pub fn remove_authorization(&mut self, node: String) {
        self.only_owner();

        self.authorized_nodes.remove(&node);
    }


    pub fn withdraw(&mut self, receipient: String, amount: u128) {
        self.only_owner();
        self.has_available_funds(amount);
        
        self.withdrawable_tokens -= amount;
        // TODO: Transfer LINK. Does this method make sense in NEAR?
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

    fn check_callback_address(&mut self, callback_address: &String) {
        assert!(callback_address != &LINK_TOKEN_ADDRESS, "Cannot callback to LINK")
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::MockedBlockchain;
    use near_bindgen::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
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
        }
    }

    // #[test]
    // fn set_get_message() {
    //     let context = get_context(vec![], false);
    //     testing_env!(context);
    //     let mut contract = StatusMessage::default();
    //     contract.set_status("hello".to_string());
    //     assert_eq!("hello".to_string(), contract.get_status("bob_near".to_string()).unwrap());
    // }

    // #[test]
    // fn set_unique_message() {
    //     let context = get_context(vec![], false);
    //     testing_env!(context);
    //     let mut contract = StatusMessage::default();
    //     // Unique
    //     assert!(contract.set_status("hello".to_string()));
    //     // Unique
    //     assert!(contract.set_status("hello world".to_string()));
    //     // Not unique. Same as current
    //     assert!(!contract.set_status("hello world".to_string()));
    //     // Not unique. Same as older
    //     assert!(!contract.set_status("hello".to_string()));
    //     // Unique
    //     assert!(contract.set_status("hi".to_string()));
    // }

    // #[test]
    // fn get_nonexistent_message() {
    //     let context = get_context(vec![], true);
    //     testing_env!(context);
    //     let contract = StatusMessage::default();
    //     assert_eq!(None, contract.get_status("francis.near".to_string()));
    // }
}
