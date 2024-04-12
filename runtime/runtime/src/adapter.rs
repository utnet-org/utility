use crate::unc_primitives::shard_layout::ShardUId;
use unc_crypto::PublicKey;
use unc_primitives::account::{AccessKey, Account};
use unc_primitives::hash::CryptoHash;
use unc_primitives::types::{
    AccountId, BlockHeight, EpochHeight, EpochId, EpochInfoProvider, MerkleHash,
};
use unc_primitives::version::ProtocolVersion;
use unc_primitives::views::{ChipView, ViewStateResult};
use unc_vm_runner::ContractCode;

/// Adapter for querying runtime.
pub trait ViewRuntimeAdapter {
    fn view_account(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
    ) -> Result<Account, crate::state_viewer::errors::ViewAccountError>;

    fn view_contract_code(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
    ) -> Result<ContractCode, crate::state_viewer::errors::ViewContractCodeError>;

    fn call_function(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        height: BlockHeight,
        block_timestamp: u64,
        last_block_hash: &CryptoHash,
        block_hash: &CryptoHash,
        epoch_height: EpochHeight,
        epoch_id: &EpochId,
        contract_id: &AccountId,
        method_name: &str,
        args: &[u8],
        logs: &mut Vec<String>,
        epoch_info_provider: &dyn EpochInfoProvider,
        current_protocol_version: ProtocolVersion,
    ) -> Result<Vec<u8>, crate::state_viewer::errors::CallFunctionError>;

    fn view_access_key(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<AccessKey, crate::state_viewer::errors::ViewAccessKeyError>;

    fn view_access_keys(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
    ) -> Result<Vec<(PublicKey, AccessKey)>, crate::state_viewer::errors::ViewAccessKeyError>;
    fn view_chip_list(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
    ) -> Result<Vec<ChipView>, crate::state_viewer::errors::ViewChipError>;

    fn view_state(
        &self,
        shard_uid: &ShardUId,
        state_root: MerkleHash,
        account_id: &AccountId,
        prefix: &[u8],
        include_proof: bool,
    ) -> Result<ViewStateResult, crate::state_viewer::errors::ViewStateError>;
}
