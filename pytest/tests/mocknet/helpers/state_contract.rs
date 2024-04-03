// Very simple contract that can:
// - write to the state
// - delete from the state
// Independently from that the same contract can be used as a receiver for `ft_transfer_call`.
use unc_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use unc_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use unc_sdk::collections::LookupMap;
use unc_sdk::json_types::{ValidAccountId, U128};
use unc_sdk::unc_bindgen;
use unc_sdk::PromiseOrValue;

unc_sdk::setup_alloc!();

#[unc_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StatusMessage {
    records: LookupMap<String, String>,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self { records: LookupMap::new(b"r".to_vec()) }
    }
}

#[unc_bindgen]
impl StatusMessage {
    pub fn set_state(&mut self, account_id: String, message: String) {
        self.records.insert(&account_id, &message);
    }

    pub fn delete_state(&mut self, account_id: String) {
        self.records.remove(&account_id);
    }

    pub fn get_state(&mut self, account_id: String) -> Option<String> {
        return self.records.get(&account_id);
    }
}

// Implements a callback which makes it possible to use `ft_transfer_call` with this contract as the
// receiver. The callback simply returns `1`.
#[unc_bindgen]
impl FungibleTokenReceiver for StatusMessage {
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        PromiseOrValue::Value(1.into())
    }
}
