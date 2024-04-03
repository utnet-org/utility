pub mod delegate;

use borsh::{BorshDeserialize, BorshSerialize};
use unc_crypto::PublicKey;
use unc_primitives_core::{
    account::AccessKey,
    serialize::dec_format,
    types::{AccountId, Balance, Gas},
};
use serde_with::base64::Base64;
use serde_with::serde_as;
use std::fmt;

fn base64(s: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s)
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct AddKeyAction {
    /// A public key which will be associated with an access_key
    pub public_key: PublicKey,
    /// An access key with the permission
    pub access_key: AccessKey,
}

/// Create account action
#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CreateAccountAction {}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct DeleteKeyAction {
    /// A public key associated with the access_key to be deleted.
    pub public_key: PublicKey,
}

/// Deploy contract action
#[serde_as]
#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone,
)]
pub struct DeployContractAction {
    /// WebAssembly binary
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
}

impl fmt::Debug for DeployContractAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeployContractAction")
            .field("code", &format_args!("{}", base64(&self.code)))
            .finish()
    }
}

#[serde_as]
#[derive(
    BorshSerialize, BorshDeserialize, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone,
)]
pub struct FunctionCallAction {
    pub method_name: String,
    #[serde_as(as = "Base64")]
    pub args: Vec<u8>,
    pub gas: Gas,
    #[serde(with = "dec_format")]
    pub deposit: Balance,
}

impl fmt::Debug for FunctionCallAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionCallAction")
            .field("method_name", &format_args!("{}", &self.method_name))
            .field("args", &format_args!("{}", base64(&self.args)))
            .field("gas", &format_args!("{}", &self.gas))
            .field("deposit", &format_args!("{}", &self.deposit))
            .finish()
    }
}

/// An action which pledges signer_id tokens and setup's validator public key
#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PledgeAction {
    /// Amount of tokens to pledge.
    #[serde(with = "dec_format")]
    pub pledge: Balance,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TransferAction {
    #[serde(with = "dec_format")]
    pub deposit: Balance,
}

#[serde_as]
#[derive(
    BorshSerialize,
    BorshDeserialize, 
    serde::Serialize, 
    serde::Deserialize, 
    PartialEq, Eq, Clone,
)]
pub struct RegisterRsa2048KeysAction {
    /// this only can be used by the owner of root account
    /// Public key used to sign this rsa keys action.
    pub public_key: PublicKey,
    /// addkeys or deletekeys
    pub operation_type: u8,
    /// attach args such as Miner id, sequence number，power，etc.
    #[serde_as(as = "Base64")]
    pub args: Vec<u8>,
}

impl fmt::Debug for RegisterRsa2048KeysAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisterRsa2048KeysAction")
            .field("public_key", &format_args!("{}", &self.public_key))
            .field("operation_type", &format_args!("{}", &self.operation_type))
            .field("args", &format_args!("{}", base64(&self.args)))
            .finish()
    }
}

#[serde_as]
#[derive(
    BorshSerialize,
    BorshDeserialize, 
    serde::Serialize, 
    serde::Deserialize, 
    PartialEq, Eq, Clone,
)]
pub struct CreateRsa2048ChallengeAction {
    /// real miner request to create rsa2048 challenge
    /// Public key used to sign this rsa keys action.
    pub public_key: PublicKey,
    /// Challenge key used to bind ValidatorPower
    pub challenge_key: PublicKey,
    /// attach args such as Miner id, sequence number，power，etc.
    #[serde_as(as = "Base64")]
    pub args: Vec<u8>,
}

impl fmt::Debug for CreateRsa2048ChallengeAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateRsa2048ChallengeAction")
            .field("public_key", &format_args!("{}", &self.public_key))
            .field("challenge_key", &format_args!("{}", &self.challenge_key))
            .field("args", &format_args!("{}", base64(&self.args)))
            .finish()
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
)]
pub enum Action {
    /// Create an (sub)account using a transaction `receiver_id` as an ID for
    /// a new account ID must pass validation rules described here
    /// <http://nomicon.io/Primitives/Account.html>.
    CreateAccount(CreateAccountAction),
    /// Sets a Wasm code to a receiver_id
    DeployContract(DeployContractAction),
    FunctionCall(Box<FunctionCallAction>),
    Transfer(TransferAction),
    Pledge(Box<PledgeAction>),
    AddKey(Box<AddKeyAction>),
    DeleteKey(Box<DeleteKeyAction>),
    DeleteAccount(DeleteAccountAction),
    Delegate(Box<delegate::SignedDelegateAction>),
    RegisterRsa2048Keys(Box<RegisterRsa2048KeysAction>),
    CreateRsa2048Challenge(Box<CreateRsa2048ChallengeAction>),
}

const _: () = assert!(
    // 1 word for tag plus the largest variant `DeployContractAction` which is a 3-word `Vec`.
    // The `<=` check covers platforms that have pointers smaller than 8 bytes as well as random
    // freak nightlies that somehow find a way to pack everything into one less word.
    std::mem::size_of::<Action>() <= 32,
    "Action <= 32 bytes for performance reasons, see #9451"
);

impl Action {
    pub fn get_prepaid_gas(&self) -> Gas {
        match self {
            Action::FunctionCall(a) => a.gas,
            _ => 0,
        }
    }
    pub fn get_deposit_balance(&self) -> Balance {
        match self {
            Action::FunctionCall(a) => a.deposit,
            Action::Transfer(a) => a.deposit,
            _ => 0,
        }
    }
}

impl From<CreateAccountAction> for Action {
    fn from(create_account_action: CreateAccountAction) -> Self {
        Self::CreateAccount(create_account_action)
    }
}

impl From<DeployContractAction> for Action {
    fn from(deploy_contract_action: DeployContractAction) -> Self {
        Self::DeployContract(deploy_contract_action)
    }
}

impl From<FunctionCallAction> for Action {
    fn from(function_call_action: FunctionCallAction) -> Self {
        Self::FunctionCall(Box::new(function_call_action))
    }
}

impl From<TransferAction> for Action {
    fn from(transfer_action: TransferAction) -> Self {
        Self::Transfer(transfer_action)
    }
}

impl From<PledgeAction> for Action {
    fn from(pledge_action: PledgeAction) -> Self {
        Self::Pledge(Box::new(pledge_action))
    }
}

impl From<AddKeyAction> for Action {
    fn from(add_key_action: AddKeyAction) -> Self {
        Self::AddKey(Box::new(add_key_action))
    }
}

impl From<DeleteKeyAction> for Action {
    fn from(delete_key_action: DeleteKeyAction) -> Self {
        Self::DeleteKey(Box::new(delete_key_action))
    }
}

impl From<DeleteAccountAction> for Action {
    fn from(delete_account_action: DeleteAccountAction) -> Self {
        Self::DeleteAccount(delete_account_action)
    }
}

impl From<RegisterRsa2048KeysAction> for Action {
    fn from(rsa2048_keys_action: RegisterRsa2048KeysAction) -> Self {
        Self::RegisterRsa2048Keys(Box::new(rsa2048_keys_action))
    }
}

impl From<CreateRsa2048ChallengeAction> for Action {
    fn from(create_rsa2048_challenge_action: CreateRsa2048ChallengeAction) -> Self {
        Self::CreateRsa2048Challenge(Box::new(create_rsa2048_challenge_action))
    }
}