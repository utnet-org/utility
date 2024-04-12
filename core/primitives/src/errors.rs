use crate::hash::CryptoHash;
use crate::serialize::dec_format;
use crate::types::{AccountId, Balance, EpochId, Gas, Nonce};
use borsh::{BorshDeserialize, BorshSerialize};
use std::fmt::{Debug, Display};
use unc_crypto::PublicKey;
use unc_primitives_core::types::{BlockHeight, ProtocolVersion};
use unc_rpc_error_macro::RpcError;

/// Error returned in the ExecutionOutcome in case of failure
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum TxExecutionError {
    /// An error happened during Action execution
    ActionError(ActionError),
    /// An error happened during Transaction execution
    InvalidTxError(InvalidTxError),
}

impl std::error::Error for TxExecutionError {}

impl Display for TxExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            TxExecutionError::ActionError(e) => write!(f, "{}", e),
            TxExecutionError::InvalidTxError(e) => write!(f, "{}", e),
        }
    }
}

impl From<ActionError> for TxExecutionError {
    fn from(error: ActionError) -> Self {
        TxExecutionError::ActionError(error)
    }
}

impl From<InvalidTxError> for TxExecutionError {
    fn from(error: InvalidTxError) -> Self {
        TxExecutionError::InvalidTxError(error)
    }
}

/// Error returned from `Runtime::apply`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// An unexpected integer overflow occurred. The likely issue is an invalid state or the transition.
    UnexpectedIntegerOverflow,
    /// An error happened during TX verification and account charging. It's likely the chunk is invalid.
    /// and should be challenged.
    InvalidTxError(InvalidTxError),
    /// Unexpected error which is typically related to the node storage corruption.
    /// It's possible the input state is invalid or malicious.
    StorageError(StorageError),
    /// An error happens if `check_balance` fails, which is likely an indication of an invalid state.
    BalanceMismatchError(Box<BalanceMismatchError>),
    /// The incoming receipt didn't pass the validation, it's likely a malicious behaviour.
    ReceiptValidationError(ReceiptValidationError),
    /// Error when accessing validator information. Happens inside epoch manager.
    ValidatorError(EpochError),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!("{:?}", self))
    }
}

impl std::error::Error for RuntimeError {}

/// Contexts in which `StorageError::MissingTrieValue` error might occur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingTrieValueContext {
    /// Missing trie value when reading from TrieIterator.
    TrieIterator,
    /// Missing trie value when reading from TriePrefetchingStorage.
    TriePrefetchingStorage,
    /// Missing trie value when reading from TrieMemoryPartialStorage.
    TrieMemoryPartialStorage,
    /// Missing trie value when reading from TrieStorage.
    TrieStorage,
}

/// Errors which may occur during working with trie storages, storing
/// trie values (trie nodes and state values) by their hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Key-value db internal failure
    StorageInternalError,
    /// Requested trie value by its hash which is missing in storage.
    MissingTrieValue(MissingTrieValueContext, CryptoHash),
    /// Found trie node which shouldn't be part of state. Raised during
    /// validation of state sync parts where incorrect node was passed.
    /// TODO (#8997): consider including hash of trie node.
    UnexpectedTrieValue,
    /// Either invalid state or key-value db is corrupted.
    /// For PartialStorage it cannot be corrupted.
    /// Error message is unreliable and for debugging purposes only. It's also probably ok to
    /// panic in every place that produces this error.
    /// We can check if db is corrupted by verifying everything in the state trie.
    StorageInconsistentState(String),
    /// Flat storage error, meaning that it doesn't support some block anymore.
    /// We guarantee that such block cannot become final, thus block processing
    /// must resume normally.
    FlatStorageBlockNotSupported(String),
    /// In-memory trie could not be loaded for some reason.
    MemTrieLoadingError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!("{:?}", self))
    }
}

impl std::error::Error for StorageError {}

/// An error happened during TX execution
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum InvalidTxError {
    /// Happens if a wrong AccessKey used or AccessKey has not enough permissions
    InvalidAccessKeyError(InvalidAccessKeyError),
    /// TX signer_id is not a valid [`AccountId`]
    InvalidSignerId { signer_id: String },
    /// TX signer_id is not found in a storage
    SignerDoesNotExist { signer_id: AccountId },
    /// Transaction nonce must be `account[access_key].nonce + 1`.
    InvalidNonce { tx_nonce: Nonce, ak_nonce: Nonce },
    /// Transaction nonce is larger than the upper bound given by the block height
    NonceTooLarge { tx_nonce: Nonce, upper_bound: Nonce },
    /// TX receiver_id is not a valid AccountId
    InvalidReceiverId { receiver_id: String },
    /// TX signature is not valid
    InvalidSignature,
    /// Account does not have enough balance to cover TX cost
    NotEnoughBalance {
        signer_id: AccountId,
        #[serde(with = "dec_format")]
        balance: Balance,
        #[serde(with = "dec_format")]
        cost: Balance,
    },
    /// Signer account doesn't have enough balance after transaction.
    LackBalanceForState {
        /// An account which doesn't have enough balance to cover storage.
        signer_id: AccountId,
        /// Required balance to cover the state.
        #[serde(with = "dec_format")]
        amount: Balance,
    },
    /// An integer overflow occurred during transaction cost estimation.
    CostOverflow,
    /// Transaction parent block hash doesn't belong to the current chain
    InvalidChain,
    /// Transaction has expired
    Expired,
    /// An error occurred while validating actions of a Transaction.
    ActionsValidation(ActionsValidationError),
    /// The size of serialized transaction exceeded the limit.
    TransactionSizeExceeded { size: u64, limit: u64 },
}

impl std::error::Error for InvalidTxError {}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum InvalidAccessKeyError {
    /// The access key identified by the `public_key` doesn't exist for the account
    AccessKeyNotFound { account_id: AccountId, public_key: Box<PublicKey> },
    /// Transaction `receiver_id` doesn't match the access key receiver_id
    ReceiverMismatch { tx_receiver: AccountId, ak_receiver: String },
    /// Transaction method name isn't allowed by the access key
    MethodNameMismatch { method_name: String },
    /// Transaction requires a full permission access key.
    RequiresFullAccess,
    /// Access Key does not have enough allowance to cover transaction cost
    NotEnoughAllowance {
        account_id: AccountId,
        public_key: Box<PublicKey>,
        #[serde(with = "dec_format")]
        allowance: Balance,
        #[serde(with = "dec_format")]
        cost: Balance,
    },
    /// Having a deposit with a function call action is not allowed with a function call access key.
    DepositWithFunctionCall,
}

/// Describes the error for validating a list of actions.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ActionsValidationError {
    /// The delete action must be a final aciton in transaction
    DeleteActionMustBeFinal,
    /// The total prepaid gas (for all given actions) exceeded the limit.
    TotalPrepaidGasExceeded { total_prepaid_gas: Gas, limit: Gas },
    /// The number of actions exceeded the given limit.
    TotalNumberOfActionsExceeded { total_number_of_actions: u64, limit: u64 },
    /// The total number of bytes of the method names exceeded the limit in a Add Key action.
    AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes: u64, limit: u64 },
    /// The length of some method name exceeded the limit in a Add Key action.
    AddKeyMethodNameLengthExceeded { length: u64, limit: u64 },
    /// Integer overflow during a compute.
    IntegerOverflow,
    /// Invalid account ID.
    InvalidAccountId { account_id: String },
    /// The size of the contract code exceeded the limit in a DeployContract action.
    ContractSizeExceeded { size: u64, limit: u64 },
    /// The length of the method name exceeded the limit in a Function Call action.
    FunctionCallMethodNameLengthExceeded { length: u64, limit: u64 },
    /// The length of the arguments exceeded the limit in a Function Call action.
    FunctionCallArgumentsLengthExceeded { length: u64, limit: u64 },
    /// An attempt to pledge with a public key that is not convertible to ristretto.
    UnsuitablePledgingKey { public_key: Box<PublicKey> },
    /// The attached amount of gas in a FunctionCall action has to be a positive number.
    FunctionCallZeroAttachedGas,
    /// There should be the only one DelegateAction
    DelegateActionMustBeOnlyOne,
    /// The transaction includes a feature that the current protocol version
    /// does not support.
    ///
    /// Note: we stringify the protocol feature name instead of using
    /// `ProtocolFeature` here because we don't want to leak the internals of
    /// that type into observable borsh serialization.
    UnsupportedProtocolFeature { protocol_feature: String, version: ProtocolVersion },
}

/// Describes the error for validating a receipt.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ReceiptValidationError {
    /// The `predecessor_id` of a Receipt is not valid.
    InvalidPredecessorId { account_id: String },
    /// The `receiver_id` of a Receipt is not valid.
    InvalidReceiverId { account_id: String },
    /// The `signer_id` of an ActionReceipt is not valid.
    InvalidSignerId { account_id: String },
    /// The `receiver_id` of a DataReceiver within an ActionReceipt is not valid.
    InvalidDataReceiverId { account_id: String },
    /// The length of the returned data exceeded the limit in a DataReceipt.
    ReturnedValueLengthExceeded { length: u64, limit: u64 },
    /// The number of input data dependencies exceeds the limit in an ActionReceipt.
    NumberInputDataDependenciesExceeded { number_of_input_data_dependencies: u64, limit: u64 },
    /// An error occurred while validating actions of an ActionReceipt.
    ActionsValidation(ActionsValidationError),
}

impl Display for ReceiptValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ReceiptValidationError::InvalidPredecessorId { account_id } => {
                write!(f, "The predecessor_id `{}` of a Receipt is not valid.", account_id)
            }
            ReceiptValidationError::InvalidReceiverId { account_id } => {
                write!(f, "The receiver_id `{}` of a Receipt is not valid.", account_id)
            }
            ReceiptValidationError::InvalidSignerId { account_id } => {
                write!(f, "The signer_id `{}` of an ActionReceipt is not valid.", account_id)
            }
            ReceiptValidationError::InvalidDataReceiverId { account_id } => write!(
                f,
                "The receiver_id `{}` of a DataReceiver within an ActionReceipt is not valid.",
                account_id
            ),
            ReceiptValidationError::ReturnedValueLengthExceeded { length, limit } => write!(
                f,
                "The length of the returned data {} exceeded the limit {} in a DataReceipt",
                length, limit
            ),
            ReceiptValidationError::NumberInputDataDependenciesExceeded { number_of_input_data_dependencies, limit } => write!(
                f,
                "The number of input data dependencies {} exceeded the limit {} in an ActionReceipt",
                number_of_input_data_dependencies, limit
            ),
            ReceiptValidationError::ActionsValidation(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ReceiptValidationError {}

impl Display for ActionsValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ActionsValidationError::DeleteActionMustBeFinal => {
                write!(f, "The delete action must be the last action in transaction")
            }
            ActionsValidationError::TotalPrepaidGasExceeded { total_prepaid_gas, limit } => {
                write!(f, "The total prepaid gas {} exceeds the limit {}", total_prepaid_gas, limit)
            }
            ActionsValidationError::TotalNumberOfActionsExceeded {total_number_of_actions, limit } => {
                write!(
                    f,
                    "The total number of actions {} exceeds the limit {}",
                    total_number_of_actions, limit
                )
            }
            ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded { total_number_of_bytes, limit } => write!(
                f,
                "The total number of bytes in allowed method names {} exceeds the maximum allowed number {} in a AddKey action",
                total_number_of_bytes, limit
            ),
            ActionsValidationError::AddKeyMethodNameLengthExceeded { length, limit } => write!(
                f,
                "The length of some method name {} exceeds the maximum allowed length {} in a AddKey action",
                length, limit
            ),
            ActionsValidationError::IntegerOverflow => write!(
                f,
                "Integer overflow during a compute",
            ),
            ActionsValidationError::InvalidAccountId { account_id } => write!(
                f,
                "Invalid account ID `{}`",
                account_id
            ),
            ActionsValidationError::ContractSizeExceeded { size, limit } => write!(
                f,
                "The length of the contract size {} exceeds the maximum allowed size {} in a DeployContract action",
                size, limit
            ),
            ActionsValidationError::FunctionCallMethodNameLengthExceeded { length, limit } => write!(
                f,
                "The length of the method name {} exceeds the maximum allowed length {} in a FunctionCall action",
                length, limit
            ),
            ActionsValidationError::FunctionCallArgumentsLengthExceeded { length, limit } => write!(
                f,
                "The length of the arguments {} exceeds the maximum allowed length {} in a FunctionCall action",
                length, limit
            ),
            ActionsValidationError::UnsuitablePledgingKey { public_key } => write!(
                f,
                "The staking key must be ristretto compatible ED25519 key. {} is provided instead.",
                public_key,
            ),
            ActionsValidationError::FunctionCallZeroAttachedGas => write!(
                f,
                "The attached amount of gas in a FunctionCall action has to be a positive number",
            ),
            ActionsValidationError::DelegateActionMustBeOnlyOne => write!(
                f,
                "The actions can contain the ony one DelegateAction"
            ),
            ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version } => write!(
                    f,
                    "Transaction requires protocol feature {} / version {} which is not supported by the current protocol version",
                    protocol_feature,
                    version,
            ),
        }
    }
}

impl std::error::Error for ActionsValidationError {}

/// An error happened during Action execution
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct ActionError {
    /// Index of the failed action in the transaction.
    /// Action index is not defined if ActionError.kind is `ActionErrorKind::LackBalanceForState`
    pub index: Option<u64>,
    /// The kind of ActionError happened
    pub kind: ActionErrorKind,
}

impl std::error::Error for ActionError {}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub enum ActionErrorKind {
    /// Happens when CreateAccount action tries to create an account with account_id which is already exists in the storage
    AccountAlreadyExists { account_id: AccountId },
    /// Happens when TX receiver_id doesn't exist (but action is not Action::CreateAccount)
    AccountDoesNotExist { account_id: AccountId },
    /// A newly created account must be under a namespace of the creator account
    CreateAccountNotAllowed { account_id: AccountId, predecessor_id: AccountId },
    /// Administrative actions like `DeployContract`, `Pledge`, `AddKey`, `DeleteKey`. can be proceed only if sender=receiver
    /// or the first TX action is a `CreateAccount` action
    ActorNoPermission { account_id: AccountId, actor_id: AccountId },
    /// Account tries to remove an access key that doesn't exist
    DeleteKeyDoesNotExist { account_id: AccountId, public_key: Box<PublicKey> },
    /// The public key is already used for an existing access key
    AddKeyAlreadyExists { account_id: AccountId, public_key: Box<PublicKey> },
    /// Account is staking and can not be deleted
    DeleteAccountPledging { account_id: AccountId },
    /// ActionReceipt can't be completed, because the remaining balance will not be enough to cover storage.
    LackBalanceForState {
        /// An account which needs balance
        account_id: AccountId,
        /// Balance required to complete an action.
        #[serde(with = "dec_format")]
        amount: Balance,
    },
    /// Account is not yet pledging, but tries to unpledge
    TriesToUnpledge { account_id: AccountId },
    /// The account doesn't have enough balance to increase the pledge.
    TriesToPledge {
        account_id: AccountId,
        #[serde(with = "dec_format")]
        pledge: Balance,
        #[serde(with = "dec_format")]
        pledging: Balance,
        #[serde(with = "dec_format")]
        balance: Balance,
    },
    InsufficientPledge {
        account_id: AccountId,
        #[serde(with = "dec_format")]
        pledge: Balance,
        #[serde(with = "dec_format")]
        minimum_pledge: Balance,
    },
    /// An error occurred during a `FunctionCall` Action, parameter is debug message.
    FunctionCallError(FunctionCallError),
    /// Error occurs when a new `ActionReceipt` created by the `FunctionCall` action fails
    /// receipt validation.
    NewReceiptValidationError(ReceiptValidationError),
    /// Error occurs when a `CreateAccount` action is called on a UNC-implicit or ETH-implicit account.
    /// See UNC-implicit account creation NEP: <https://github.com/utility/UEPs/pull/71>.
    /// Also, see ETH-implicit account creation NEP: <https://github.com/Utility/UEPs/issues/518>.
    ///
    /// TODO(#8598): This error is named very poorly. A better name would be
    /// `OnlyReservedCreationAllowed`.
    OnlyImplicitAccountCreationAllowed { account_id: AccountId },
    /// Delete account whose state is large is temporarily banned.
    DeleteAccountWithLargeState { account_id: AccountId },
    /// Signature does not match the provided actions and given signer public key.
    DelegateActionInvalidSignature,
    /// Receiver of the transaction doesn't match Sender of the delegate action
    DelegateActionSenderDoesNotMatchTxReceiver { sender_id: AccountId, receiver_id: AccountId },
    /// Delegate action has expired. `max_block_height` is less than actual block height.
    DelegateActionExpired,
    /// The given public key doesn't exist for Sender account
    DelegateActionAccessKeyError(InvalidAccessKeyError),
    /// DelegateAction nonce must be greater sender[public_key].nonce
    DelegateActionInvalidNonce { delegate_nonce: Nonce, ak_nonce: Nonce },
    /// DelegateAction nonce is larger than the upper bound given by the block height
    DelegateActionNonceTooLarge { delegate_nonce: Nonce, upper_bound: Nonce },

    /// The public key used for an  not existed  rsa key
    RsaKeysNotFound { account_id: AccountId, public_key: Box<PublicKey> },
}

impl From<ActionErrorKind> for ActionError {
    fn from(e: ActionErrorKind) -> ActionError {
        ActionError { index: None, kind: e }
    }
}

impl Display for InvalidTxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            InvalidTxError::InvalidSignerId { signer_id } => {
                write!(f, "Invalid signer account ID {:?} according to requirements", signer_id)
            }
            InvalidTxError::SignerDoesNotExist { signer_id } => {
                write!(f, "Signer {:?} does not exist", signer_id)
            }
            InvalidTxError::InvalidAccessKeyError(access_key_error) => {
                Display::fmt(&access_key_error, f)
            }
            InvalidTxError::InvalidNonce { tx_nonce, ak_nonce } => write!(
                f,
                "Transaction nonce {} must be larger than nonce of the used access key {}",
                tx_nonce, ak_nonce
            ),
            InvalidTxError::InvalidReceiverId { receiver_id } => {
                write!(f, "Invalid receiver account ID {:?} according to requirements", receiver_id)
            }
            InvalidTxError::InvalidSignature => {
                write!(f, "Transaction is not signed with the given public key")
            }
            InvalidTxError::NotEnoughBalance { signer_id, balance, cost } => write!(
                f,
                "Sender {:?} does not have enough balance {} for operation costing {}",
                signer_id, balance, cost
            ),
            InvalidTxError::LackBalanceForState { signer_id, amount } => {
                write!(f, "Failed to execute, because the account {:?} wouldn't have enough balance to cover storage, required to have {} attoUNC more", signer_id, amount)
            }
            InvalidTxError::CostOverflow => {
                write!(f, "Transaction gas or balance cost is too high")
            }
            InvalidTxError::InvalidChain => {
                write!(f, "Transaction parent block hash doesn't belong to the current chain")
            }
            InvalidTxError::Expired => {
                write!(f, "Transaction has expired")
            }
            InvalidTxError::ActionsValidation(error) => {
                write!(f, "Transaction actions validation error: {}", error)
            }
            InvalidTxError::NonceTooLarge { tx_nonce, upper_bound } => {
                write!(
                    f,
                    "Transaction nonce {} must be smaller than the access key nonce upper bound {}",
                    tx_nonce, upper_bound
                )
            }
            InvalidTxError::TransactionSizeExceeded { size, limit } => {
                write!(f, "Size of serialized transaction {} exceeded the limit {}", size, limit)
            }
        }
    }
}

impl From<InvalidAccessKeyError> for InvalidTxError {
    fn from(error: InvalidAccessKeyError) -> Self {
        InvalidTxError::InvalidAccessKeyError(error)
    }
}

impl Display for InvalidAccessKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            InvalidAccessKeyError::AccessKeyNotFound { account_id, public_key } => write!(
                f,
                "Signer {:?} doesn't have access key with the given public_key {}",
                account_id, public_key
            ),
            InvalidAccessKeyError::ReceiverMismatch { tx_receiver, ak_receiver } => write!(
                f,
                "Transaction receiver_id {:?} doesn't match the access key receiver_id {:?}",
                tx_receiver, ak_receiver
            ),
            InvalidAccessKeyError::MethodNameMismatch { method_name } => write!(
                f,
                "Transaction method name {:?} isn't allowed by the access key",
                method_name
            ),
            InvalidAccessKeyError::RequiresFullAccess => {
                write!(f, "Invalid access key type. Full-access keys are required for transactions that have multiple or non-function-call actions")
            }
            InvalidAccessKeyError::NotEnoughAllowance {
                account_id,
                public_key,
                allowance,
                cost,
            } => write!(
                f,
                "Access Key {:?}:{} does not have enough balance {} for transaction costing {}",
                account_id, public_key, allowance, cost
            ),
            InvalidAccessKeyError::DepositWithFunctionCall => {
                write!(f, "Having a deposit with a function call action is not allowed with a function call access key.")
            }
        }
    }
}

impl std::error::Error for InvalidAccessKeyError {}

/// Happens when the input balance doesn't match the output balance in Runtime apply.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct BalanceMismatchError {
    // Input balances
    #[serde(with = "dec_format")]
    pub incoming_validator_rewards: Balance,
    #[serde(with = "dec_format")]
    pub initial_accounts_balance: Balance,
    #[serde(with = "dec_format")]
    pub incoming_receipts_balance: Balance,
    #[serde(with = "dec_format")]
    pub processed_delayed_receipts_balance: Balance,
    #[serde(with = "dec_format")]
    pub initial_postponed_receipts_balance: Balance,
    // Output balances
    #[serde(with = "dec_format")]
    pub final_accounts_balance: Balance,
    #[serde(with = "dec_format")]
    pub outgoing_receipts_balance: Balance,
    #[serde(with = "dec_format")]
    pub new_delayed_receipts_balance: Balance,
    #[serde(with = "dec_format")]
    pub final_postponed_receipts_balance: Balance,
    #[serde(with = "dec_format")]
    pub tx_burnt_amount: Balance,
    #[serde(with = "dec_format")]
    pub slashed_burnt_amount: Balance,
    #[serde(with = "dec_format")]
    pub other_burnt_amount: Balance,
}

impl Display for BalanceMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // Using saturating add to avoid overflow in display
        let initial_balance = self
            .incoming_validator_rewards
            .saturating_add(self.initial_accounts_balance)
            .saturating_add(self.incoming_receipts_balance)
            .saturating_add(self.processed_delayed_receipts_balance)
            .saturating_add(self.initial_postponed_receipts_balance);
        let final_balance = self
            .final_accounts_balance
            .saturating_add(self.outgoing_receipts_balance)
            .saturating_add(self.new_delayed_receipts_balance)
            .saturating_add(self.final_postponed_receipts_balance)
            .saturating_add(self.tx_burnt_amount)
            .saturating_add(self.slashed_burnt_amount)
            .saturating_add(self.other_burnt_amount);
        write!(
            f,
            "Balance Mismatch Error. The input balance {} doesn't match output balance {}\n\
             Inputs:\n\
             \tIncoming validator rewards sum: {}\n\
             \tInitial accounts balance sum: {}\n\
             \tIncoming receipts balance sum: {}\n\
             \tProcessed delayed receipts balance sum: {}\n\
             \tInitial postponed receipts balance sum: {}\n\
             Outputs:\n\
             \tFinal accounts balance sum: {}\n\
             \tOutgoing receipts balance sum: {}\n\
             \tNew delayed receipts balance sum: {}\n\
             \tFinal postponed receipts balance sum: {}\n\
             \tTx fees burnt amount: {}\n\
             \tSlashed amount: {}\n\
             \tOther burnt amount: {}",
            initial_balance,
            final_balance,
            self.incoming_validator_rewards,
            self.initial_accounts_balance,
            self.incoming_receipts_balance,
            self.processed_delayed_receipts_balance,
            self.initial_postponed_receipts_balance,
            self.final_accounts_balance,
            self.outgoing_receipts_balance,
            self.new_delayed_receipts_balance,
            self.final_postponed_receipts_balance,
            self.tx_burnt_amount,
            self.slashed_burnt_amount,
            self.other_burnt_amount,
        )
    }
}

impl std::error::Error for BalanceMismatchError {}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct IntegerOverflowError;

impl std::fmt::Display for IntegerOverflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&format!("{:?}", self))
    }
}

impl std::error::Error for IntegerOverflowError {}

impl From<IntegerOverflowError> for InvalidTxError {
    fn from(_: IntegerOverflowError) -> Self {
        InvalidTxError::CostOverflow
    }
}

impl From<IntegerOverflowError> for RuntimeError {
    fn from(_: IntegerOverflowError) -> Self {
        RuntimeError::UnexpectedIntegerOverflow
    }
}

impl From<StorageError> for RuntimeError {
    fn from(e: StorageError) -> Self {
        RuntimeError::StorageError(e)
    }
}

impl From<BalanceMismatchError> for RuntimeError {
    fn from(e: BalanceMismatchError) -> Self {
        RuntimeError::BalanceMismatchError(Box::new(e))
    }
}

impl From<InvalidTxError> for RuntimeError {
    fn from(e: InvalidTxError) -> Self {
        RuntimeError::InvalidTxError(e)
    }
}

impl From<EpochError> for RuntimeError {
    fn from(e: EpochError) -> Self {
        RuntimeError::ValidatorError(e)
    }
}

impl Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Action #{}: {}", self.index.unwrap_or_default(), self.kind)
    }
}

impl Display for ActionErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ActionErrorKind::AccountAlreadyExists { account_id } => {
                write!(f, "Can't create a new account {:?}, because it already exists", account_id)
            }
            ActionErrorKind::AccountDoesNotExist { account_id } => write!(
                f,
                "Can't complete the action because account {:?} doesn't exist",
                account_id
            ),
            ActionErrorKind::ActorNoPermission { actor_id, account_id } => write!(
                f,
                "Actor {:?} doesn't have permission to account {:?} to complete the action",
                actor_id, account_id
            ),
            ActionErrorKind::LackBalanceForState { account_id, amount } => write!(
                f,
                "The account {} wouldn't have enough balance to cover storage, required to have {} attoUNC more",
                account_id, amount
            ),
            ActionErrorKind::TriesToUnpledge { account_id } => {
                write!(f, "Account {:?} is not yet pledging, but tries to unpledge", account_id)
            }
            ActionErrorKind::TriesToPledge { account_id, pledge, pledging, balance } => write!(
                f,
                "Account {:?} tries to pledge {}, but has pledging {} and only has {}",
                account_id, pledge, pledging, balance
            ),
            ActionErrorKind::CreateAccountNotAllowed { account_id, predecessor_id } => write!(
                f,
                "A sub-account ID {:?} can't be created by account {:?}",
                account_id, predecessor_id,
            ),
            ActionErrorKind::DeleteKeyDoesNotExist { account_id, .. } => write!(
                f,
                "Account {:?} tries to remove an access key that doesn't exist",
                account_id
            ),
            ActionErrorKind::AddKeyAlreadyExists { public_key, .. } => write!(
                f,
                "The public key {:?} is already used for an existing access key",
                public_key
            ),
            ActionErrorKind::DeleteAccountPledging{ account_id } => {
                write!(f, "Account {:?} is staking and can not be deleted", account_id)
            }
            ActionErrorKind::FunctionCallError(s) => write!(f, "{:?}", s),
            ActionErrorKind::NewReceiptValidationError(e) => {
                write!(f, "An new action receipt created during a FunctionCall is not valid: {}", e)
            }
            ActionErrorKind::InsufficientPledge { account_id, pledge, minimum_pledge } => write!(f, "Account {} tries to pledge {} but minimum required pledge is {}", account_id, pledge, minimum_pledge),
            ActionErrorKind::OnlyImplicitAccountCreationAllowed { account_id } => write!(f, "CreateAccount action is called on hex-characters account of length 64 {}", account_id),
            ActionErrorKind::DeleteAccountWithLargeState { account_id } => write!(f, "The state of account {} is too large and therefore cannot be deleted", account_id),
            ActionErrorKind::DelegateActionInvalidSignature => write!(f, "DelegateAction is not signed with the given public key"),
            ActionErrorKind::DelegateActionSenderDoesNotMatchTxReceiver { sender_id, receiver_id } => write!(f, "Transaction receiver {} doesn't match DelegateAction sender {}", receiver_id, sender_id),
            ActionErrorKind::DelegateActionExpired => write!(f, "DelegateAction has expired"),
            ActionErrorKind::DelegateActionAccessKeyError(access_key_error) => Display::fmt(&access_key_error, f),
            ActionErrorKind::DelegateActionInvalidNonce { delegate_nonce, ak_nonce } => write!(f, "DelegateAction nonce {} must be larger than nonce of the used access key {}", delegate_nonce, ak_nonce),
            ActionErrorKind::DelegateActionNonceTooLarge { delegate_nonce, upper_bound } => write!(f, "DelegateAction nonce {} must be smaller than the access key nonce upper bound {}", delegate_nonce, upper_bound),
            ActionErrorKind::RsaKeysNotFound { public_key, .. } => write!(
                f,
                "The public key {:?} is doesn't exist rsa key",
                public_key
            ),
        }
    }
}
#[derive(Eq, PartialEq, Clone)]
pub enum BlockError {
    /// Error calculating threshold from given pledges for given number of seats.
    /// Only should happened if calling code doesn't check for integer value of pledge > number of seats.
    ThresholdError {
        pledge_sum: Balance,
        num_seats: u64,
    },
    /// Requesting validators for an epoch that wasn't computed yet.
    BlockOutOfBounds(CryptoHash),
    /// Missing block hash in the storage (means there is some structural issue).
    MissingBlock(CryptoHash),
    /// Error due to IO (DB read/write, serialization, etc.).
    IOErr(String),
    /// Given account ID is not a validator in the given block height.
    NotAValidator(AccountId, BlockHeight),
    /// Error getting information for a shard
    ShardingError(String),
    NotEnoughValidators {
        num_validators: u64,
        num_shards: u64,
    },
    /// Error selecting validators for a chunk.
    ChunkValidatorSelectionError(String),
    /// ValidatorTotalPowerError
    ValidatorTotalPowerError(String),
    /// NoAvailableValidator
    NoAvailableValidator(String),
}

impl std::error::Error for crate::errors::BlockError {}

impl Display for crate::errors::BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::errors::BlockError::ThresholdError { pledge_sum, num_seats } => write!(
                f,
                "Total pledge {} must be higher than the number of seats {}",
                pledge_sum, num_seats
            ),
            crate::errors::BlockError::BlockOutOfBounds(block_height) => {
                write!(f, "Block {:?} is out of bounds", block_height)
            }
            crate::errors::BlockError::MissingBlock(hash) => write!(f, "Missing block {}", hash),
            crate::errors::BlockError::IOErr(err) => write!(f, "IO: {}", err),
            crate::errors::BlockError::NotAValidator(account_id, block_height) => {
                write!(f, "{} is not a validator in epoch {:?}", account_id, block_height)
            }
            crate::errors::BlockError::ShardingError(err) => write!(f, "Sharding Error: {}", err),
            crate::errors::BlockError::NotEnoughValidators { num_shards, num_validators } => {
                write!(f, "There were not enough validator proposals to fill all shards. num_proposals: {}, num_shards: {}", num_validators, num_shards)
            }
            crate::errors::BlockError::ChunkValidatorSelectionError(err) => {
                write!(f, "Error selecting validators for a chunk: {}", err)
            }
            crate::errors::BlockError::ValidatorTotalPowerError(err) => {
                write!(f, "Error when computing total power: {}", err)
            }
            crate::errors::BlockError::NoAvailableValidator(err) => {
                write!(f, "Error selecting produce: {}", err)
            }
        }
    }
}

impl Debug for crate::errors::BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            crate::errors::BlockError::ThresholdError { pledge_sum, num_seats } => {
                write!(f, "ThresholdError({}, {})", pledge_sum, num_seats)
            }
            crate::errors::BlockError::BlockOutOfBounds(block_height) => {
                write!(f, "EpochOutOfBounds({:?})", block_height)
            }
            crate::errors::BlockError::MissingBlock(hash) => write!(f, "MissingBlock({})", hash),
            crate::errors::BlockError::IOErr(err) => write!(f, "IOErr({})", err),
            crate::errors::BlockError::NotAValidator(account_id, block_height) => {
                write!(f, "NotAValidator({}, {:?})", account_id, block_height)
            }
            crate::errors::BlockError::ShardingError(err) => write!(f, "ShardingError({})", err),
            crate::errors::BlockError::NotEnoughValidators { num_shards, num_validators } => {
                write!(f, "NotEnoughValidators({}, {})", num_validators, num_shards)
            }
            crate::errors::BlockError::ChunkValidatorSelectionError(err) => {
                write!(f, "ChunkValidatorSelectionError({})", err)
            }
            crate::errors::BlockError::ValidatorTotalPowerError(err) => {
                write!(f, "Error when computing total power: {}", err)
            }
            crate::errors::BlockError::NoAvailableValidator(err) => {
                write!(f, "Error selecting produce: {}", err)
            }
        }
    }
}

impl From<std::io::Error> for crate::errors::BlockError {
    fn from(error: std::io::Error) -> Self {
        crate::errors::BlockError::IOErr(error.to_string())
    }
}

impl From<EpochError> for BlockError {
    fn from(error: EpochError) -> Self {
        match error {
            EpochError::IOErr(..) => BlockError::IOErr(error.to_string()),
            EpochError::ChunkValidatorSelectionError(..) => {
                BlockError::ChunkValidatorSelectionError(error.to_string())
            }
            EpochError::EpochOutOfBounds(..) => BlockError::BlockOutOfBounds(CryptoHash::default()),
            EpochError::MissingBlock(block_hash) => BlockError::MissingBlock(block_hash),
            EpochError::NotAValidator(account_id, _hash) => {
                BlockError::NotAValidator(account_id, 0)
            }
            EpochError::NotEnoughValidators { num_validators: x, num_shards: y } => {
                BlockError::NotEnoughValidators { num_validators: x, num_shards: y }
            }
            EpochError::ShardingError(..) => BlockError::ShardingError(error.to_string()),
            EpochError::ThresholdError { pledge_sum: pledge, num_seats: seats } => {
                BlockError::ThresholdError { pledge_sum: pledge, num_seats: seats }
            }
        }
    }
}
#[derive(Eq, PartialEq, Clone)]
pub enum EpochError {
    /// Error calculating threshold from given pledges for given number of seats.
    /// Only should happened if calling code doesn't check for integer value of pledge > number of seats.
    ThresholdError {
        pledge_sum: Balance,
        num_seats: u64,
    },
    /// Requesting validators for an epoch that wasn't computed yet.
    EpochOutOfBounds(EpochId),
    /// Missing block hash in the storage (means there is some structural issue).
    MissingBlock(CryptoHash),
    /// Error due to IO (DB read/write, serialization, etc.).
    IOErr(String),
    /// Given account ID is not a validator in the given epoch ID.
    NotAValidator(AccountId, EpochId),
    /// Error getting information for a shard
    ShardingError(String),
    NotEnoughValidators {
        num_validators: u64,
        num_shards: u64,
    },
    /// Error selecting validators for a chunk.
    ChunkValidatorSelectionError(String),
}

impl std::error::Error for EpochError {}

impl Display for EpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochError::ThresholdError { pledge_sum, num_seats } => write!(
                f,
                "Total pledge {} must be higher than the number of seats {}",
                pledge_sum, num_seats
            ),
            EpochError::EpochOutOfBounds(epoch_id) => {
                write!(f, "Epoch {:?} is out of bounds", epoch_id)
            }
            EpochError::MissingBlock(hash) => write!(f, "Missing block {}", hash),
            EpochError::IOErr(err) => write!(f, "IO: {}", err),
            EpochError::NotAValidator(account_id, epoch_id) => {
                write!(f, "{} is not a validator in epoch {:?}", account_id, epoch_id)
            }
            EpochError::ShardingError(err) => write!(f, "Sharding Error: {}", err),
            EpochError::NotEnoughValidators { num_shards, num_validators } => {
                write!(f, "There were not enough validator proposals to fill all shards. num_proposals: {}, num_shards: {}", num_validators, num_shards)
            }
            EpochError::ChunkValidatorSelectionError(err) => {
                write!(f, "Error selecting validators for a chunk: {}", err)
            }
        }
    }
}

impl Debug for EpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochError::ThresholdError { pledge_sum, num_seats } => {
                write!(f, "ThresholdError({}, {})", pledge_sum, num_seats)
            }
            EpochError::EpochOutOfBounds(epoch_id) => write!(f, "EpochOutOfBounds({:?})", epoch_id),
            EpochError::MissingBlock(hash) => write!(f, "MissingBlock({})", hash),
            EpochError::IOErr(err) => write!(f, "IOErr({})", err),
            EpochError::NotAValidator(account_id, epoch_id) => {
                write!(f, "NotAValidator({}, {:?})", account_id, epoch_id)
            }
            EpochError::ShardingError(err) => write!(f, "ShardingError({})", err),
            EpochError::NotEnoughValidators { num_shards, num_validators } => {
                write!(f, "NotEnoughValidators({}, {})", num_validators, num_shards)
            }
            EpochError::ChunkValidatorSelectionError(err) => {
                write!(f, "ChunkValidatorSelectionError({})", err)
            }
        }
    }
}

impl From<std::io::Error> for EpochError {
    fn from(error: std::io::Error) -> Self {
        EpochError::IOErr(error.to_string())
    }
}

impl From<BlockError> for EpochError {
    fn from(error: BlockError) -> Self {
        match error {
            BlockError::IOErr(..) => EpochError::IOErr(error.to_string()),
            BlockError::ChunkValidatorSelectionError(..) => {
                EpochError::ChunkValidatorSelectionError(error.to_string())
            }
            BlockError::BlockOutOfBounds(..) => EpochError::EpochOutOfBounds(EpochId::default()),
            BlockError::MissingBlock(block_hash) => EpochError::MissingBlock(block_hash),
            BlockError::NotAValidator(account_id, _block_height) => {
                EpochError::NotAValidator(account_id, EpochId::default())
            }
            BlockError::NotEnoughValidators { num_validators: x, num_shards: y } => {
                EpochError::NotEnoughValidators { num_validators: x, num_shards: y }
            }
            BlockError::ShardingError(..) => EpochError::ShardingError(error.to_string()),
            BlockError::ThresholdError { pledge_sum: pledge, num_seats: seats } => {
                EpochError::ThresholdError { pledge_sum: pledge, num_seats: seats }
            }
            BlockError::ValidatorTotalPowerError(..) => EpochError::IOErr(error.to_string()),
            BlockError::NoAvailableValidator(..) => EpochError::IOErr(error.to_string()),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
)]
/// Error that can occur while preparing or executing Wasm smart-contract.
pub enum PrepareError {
    /// Error happened while serializing the module.
    Serialization,
    /// Error happened while deserializing the module.
    Deserialization,
    /// Internal memory declaration has been found in the module.
    InternalMemoryDeclared,
    /// Gas instrumentation failed.
    ///
    /// This most likely indicates the module isn't valid.
    GasInstrumentation,
    /// Stack instrumentation failed.
    ///
    /// This  most likely indicates the module isn't valid.
    StackHeightInstrumentation,
    /// Error happened during instantiation.
    ///
    /// This might indicate that `start` function trapped, or module isn't
    /// instantiable and/or unlinkable.
    Instantiate,
    /// Error creating memory.
    Memory,
    /// Contract contains too many functions.
    TooManyFunctions,
    /// Contract contains too many locals.
    TooManyLocals,
}

/// A kind of a trap happened during execution of a binary
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
    strum::IntoStaticStr,
)]
pub enum WasmTrap {
    /// An `unreachable` opcode was executed.
    Unreachable,
    /// Call indirect incorrect signature trap.
    IncorrectCallIndirectSignature,
    /// Memory out of bounds trap.
    MemoryOutOfBounds,
    /// Call indirect out of bounds trap.
    CallIndirectOOB,
    /// An arithmetic exception, e.g. divided by zero.
    IllegalArithmetic,
    /// Misaligned atomic access trap.
    MisalignedAtomicAccess,
    /// Indirect call to null.
    IndirectCallToNull,
    /// Stack overflow.
    StackOverflow,
    /// Generic trap.
    GenericTrap,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
    strum::IntoStaticStr,
)]
pub enum HostError {
    /// String encoding is bad UTF-16 sequence
    BadUTF16,
    /// String encoding is bad UTF-8 sequence
    BadUTF8,
    /// Exceeded the prepaid gas
    GasExceeded,
    /// Exceeded the maximum amount of gas allowed to burn per contract
    GasLimitExceeded,
    /// Exceeded the account balance
    BalanceExceeded,
    /// Tried to call an empty method name
    EmptyMethodName,
    /// Smart contract panicked
    GuestPanic { panic_msg: String },
    /// IntegerOverflow happened during a contract execution
    IntegerOverflow,
    /// `promise_idx` does not correspond to existing promises
    InvalidPromiseIndex { promise_idx: u64 },
    /// Actions can only be appended to non-joint promise.
    CannotAppendActionToJointPromise,
    /// Returning joint promise is currently prohibited
    CannotReturnJointPromise,
    /// Accessed invalid promise result index
    InvalidPromiseResultIndex { result_idx: u64 },
    /// Accessed invalid register id
    InvalidRegisterId { register_id: u64 },
    /// Iterator `iterator_index` was invalidated after its creation by performing a mutable operation on trie
    IteratorWasInvalidated { iterator_index: u64 },
    /// Accessed memory outside the bounds
    MemoryAccessViolation,
    /// VM Logic returned an invalid receipt index
    InvalidReceiptIndex { receipt_index: u64 },
    /// Iterator index `iterator_index` does not exist
    InvalidIteratorIndex { iterator_index: u64 },
    /// VM Logic returned an invalid account id
    InvalidAccountId,
    /// VM Logic returned an invalid method name
    InvalidMethodName,
    /// VM Logic provided an invalid public key
    InvalidPublicKey,
    /// `method_name` is not allowed in view calls
    ProhibitedInView { method_name: String },
    /// The total number of logs will exceed the limit.
    NumberOfLogsExceeded { limit: u64 },
    /// The storage key length exceeded the limit.
    KeyLengthExceeded { length: u64, limit: u64 },
    /// The storage value length exceeded the limit.
    ValueLengthExceeded { length: u64, limit: u64 },
    /// The total log length exceeded the limit.
    TotalLogLengthExceeded { length: u64, limit: u64 },
    /// The maximum number of promises within a FunctionCall exceeded the limit.
    NumberPromisesExceeded { number_of_promises: u64, limit: u64 },
    /// The maximum number of input data dependencies exceeded the limit.
    NumberInputDataDependenciesExceeded { number_of_input_data_dependencies: u64, limit: u64 },
    /// The returned value length exceeded the limit.
    ReturnedValueLengthExceeded { length: u64, limit: u64 },
    /// The contract size for DeployContract action exceeded the limit.
    ContractSizeExceeded { size: u64, limit: u64 },
    /// The host function was deprecated.
    Deprecated { method_name: String },
    /// General errors for ECDSA recover.
    ECRecoverError { msg: String },
    /// Invalid input to alt_bn128 familiy of functions (e.g., point which isn't
    /// on the curve).
    AltBn128InvalidInput { msg: String },
    /// Invalid input to ed25519 signature verification function (e.g. signature cannot be
    /// derived from bytes).
    Ed25519VerifyInvalidInput { msg: String },
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
    strum::IntoStaticStr,
)]
pub enum MethodResolveError {
    MethodEmptyName,
    MethodNotFound,
    MethodInvalidSignature,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    RpcError,
    serde::Deserialize,
    serde::Serialize,
    strum::IntoStaticStr,
)]
pub enum CompilationError {
    CodeDoesNotExist {
        account_id: AccountId,
    },
    PrepareError(PrepareError),
    /// This is for defense in depth.
    /// We expect our runtime-independent preparation code to fully catch all invalid wasms,
    /// but, if it ever misses something we’ll emit this error
    WasmerCompileError {
        msg: String,
    },
}

/// Serializable version of `unc-vm-runner::FunctionCallError`.
///
/// Must never reorder/remove elements, can only add new variants at the end (but do that very
/// carefully). It describes stable serialization format, and only used by serialization logic.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum FunctionCallError {
    /// Wasm compilation error
    CompilationError(CompilationError),
    /// Wasm binary env link error
    ///
    /// Note: this is only to deserialize old data, use execution error for new data
    LinkError {
        msg: String,
    },
    /// Import/export resolve error
    MethodResolveError(MethodResolveError),
    /// A trap happened during execution of a binary
    ///
    /// Note: this is only to deserialize old data, use execution error for new data
    WasmTrap(WasmTrap),
    WasmUnknownError,
    /// Note: this is only to deserialize old data, use execution error for new data
    HostError(HostError),
    // Unused, can be reused by a future error but must be exactly one error to keep ExecutionError
    // error borsh serialized at correct index
    _EVMError,
    ExecutionError(String),
}

impl From<unc_vm_runner::logic::errors::MethodResolveError> for MethodResolveError {
    fn from(outer_err: unc_vm_runner::logic::errors::MethodResolveError) -> Self {
        use unc_vm_runner::logic::errors::MethodResolveError as MRE;
        match outer_err {
            MRE::MethodEmptyName => Self::MethodEmptyName,
            MRE::MethodNotFound => Self::MethodNotFound,
            MRE::MethodInvalidSignature => Self::MethodInvalidSignature,
        }
    }
}

impl From<unc_vm_runner::logic::errors::PrepareError> for PrepareError {
    fn from(outer_err: unc_vm_runner::logic::errors::PrepareError) -> Self {
        use unc_vm_runner::logic::errors::PrepareError as PE;
        match outer_err {
            PE::Serialization => Self::Serialization,
            PE::Deserialization => Self::Deserialization,
            PE::InternalMemoryDeclared => Self::InternalMemoryDeclared,
            PE::GasInstrumentation => Self::GasInstrumentation,
            PE::StackHeightInstrumentation => Self::StackHeightInstrumentation,
            PE::Instantiate => Self::Instantiate,
            PE::Memory => Self::Memory,
            PE::TooManyFunctions => Self::TooManyFunctions,
            PE::TooManyLocals => Self::TooManyLocals,
        }
    }
}

impl From<unc_vm_runner::logic::errors::CompilationError> for CompilationError {
    fn from(outer_err: unc_vm_runner::logic::errors::CompilationError) -> Self {
        use unc_vm_runner::logic::errors::CompilationError as CE;
        match outer_err {
            CE::CodeDoesNotExist { account_id } => Self::CodeDoesNotExist {
                account_id: account_id.parse().expect("account_id in error must be valid"),
            },
            CE::PrepareError(pe) => Self::PrepareError(pe.into()),
            CE::WasmerCompileError { msg } => Self::WasmerCompileError { msg },
        }
    }
}

impl From<unc_vm_runner::logic::errors::FunctionCallError> for FunctionCallError {
    fn from(outer_err: unc_vm_runner::logic::errors::FunctionCallError) -> Self {
        use unc_vm_runner::logic::errors::FunctionCallError as FCE;
        match outer_err {
            FCE::CompilationError(e) => Self::CompilationError(e.into()),
            FCE::MethodResolveError(e) => Self::MethodResolveError(e.into()),
            // Note: We deliberately collapse all execution errors for
            // serialization to make the DB representation less dependent
            // on specific types in Rust code.
            FCE::HostError(ref _e) => Self::ExecutionError(outer_err.to_string()),
            FCE::LinkError { msg } => Self::ExecutionError(format!("Link Error: {}", msg)),
            FCE::WasmTrap(ref _e) => Self::ExecutionError(outer_err.to_string()),
        }
    }
}

#[cfg(feature = "new_epoch_sync")]
pub mod epoch_sync {
    use std::fmt::Debug;
    use unc_primitives_core::hash::CryptoHash;
    use unc_primitives_core::types::EpochHeight;

    #[derive(Eq, PartialEq, Clone, strum::Display, Debug)]
    pub enum EpochSyncHashType {
        LastEpochBlock,
        LastFinalBlock,
        FirstEpochBlock,
        NextEpochFirstBlock,
        Other,
        BlockToSave,
    }

    #[derive(Eq, PartialEq, Clone, thiserror::Error, Debug)]
    pub enum EpochSyncInfoError {
        #[error("{hash_type} hash {hash:?} not a part of EpochSyncInfo for epoch {epoch_height}")]
        HashNotFound { hash: CryptoHash, hash_type: EpochSyncHashType, epoch_height: EpochHeight },
        #[error("all_block_hashes.len() < 2 for epoch {epoch_height}")]
        ShortEpoch { epoch_height: EpochHeight },
    }
}
