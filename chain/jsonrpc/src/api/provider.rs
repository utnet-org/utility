use serde_json::Value;
use unc_primitives::types::EpochId;

use unc_client_primitives::types::{GetProviderError, GetProviderInfoError};
use unc_jsonrpc_primitives::errors::RpcParseError;
use unc_jsonrpc_primitives::types::provider::{RpcProviderError, RpcProviderRequest};

use super::{RpcFrom, RpcRequest};

impl RpcRequest for RpcProviderRequest {
    // fn parse(value: Value) -> Result<Self, RpcParseError> {
    //     let block_height = value
    //             .get("block_height")
    //             .and_then(|v| v.as_u64())
    //             .ok_or_else(|| RpcParseError("block_height not found or not a u64".parse().unwrap()))?;
    //     Ok(Self { block_height })
    // }
    fn parse(value: Value) -> Result<Self, RpcParseError> {
        // Extract block_hash_str from value
        let epoch_id_str = value
            .get("epoch_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcParseError("epoch_id not found or not a string".parse().unwrap()))?;

        // Decode the base58-encoded string to bytes
        let bytes = bs58::decode(epoch_id_str)
            .into_vec()
            .map_err(|_| RpcParseError("Invalid base58-encoded hash".parse().unwrap()))?;

        // Ensure the decoded bytes have the correct length for a CryptoHash
        if bytes.len() != 32 {
            return Err(RpcParseError(
                "Decoded hash does not match expected length".parse().unwrap(),
            ));
        }

        // Construct the CryptoHash from the decoded bytes
        let epoch_id: EpochId = epoch_id_str
            .parse()
            .map_err(|_| RpcParseError("Failed to parse epoch_id from base58".parse().unwrap()))?;

        let block_height = value
            .get("block_height")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| RpcParseError("block_height not found or not a u64".parse().unwrap()))?;

        Ok(Self { epoch_id, block_height })
    }
}

impl RpcFrom<actix::MailboxError> for RpcProviderError {
    fn rpc_from(error: actix::MailboxError) -> Self {
        Self::InternalError { error_message: error.to_string() }
    }
}

impl RpcFrom<GetProviderInfoError> for RpcProviderError {
    fn rpc_from(error: GetProviderInfoError) -> Self {
        match error {
            GetProviderInfoError::UnknownBlock => Self::UnknownBlock,
            GetProviderInfoError::ProviderInfoUnavailable => Self::ProviderInfoUnavailable,
            GetProviderInfoError::IOError(error_message) => Self::InternalError { error_message },
            GetProviderInfoError::Unreachable(ref error_message) => {
                tracing::warn!(target: "jsonrpc", "Unreachable error occurred: {}", error_message);
                crate::metrics::RPC_UNREACHABLE_ERROR_COUNT
                    .with_label_values(&["RpcProviderError"])
                    .inc();
                Self::InternalError { error_message: error.to_string() }
            }
        }
    }
}

impl RpcFrom<GetProviderError> for RpcProviderError {
    fn rpc_from(error: GetProviderError) -> Self {
        match error {
            GetProviderError::UnknownBlock { .. } => Self::UnknownBlock {},
            GetProviderError::NotSyncedYet { .. } => Self::ProviderInfoUnavailable,
            GetProviderError::IOError { error_message } => Self::InternalError { error_message },
            GetProviderError::Unreachable { ref error_message } => {
                tracing::warn!(target: "jsonrpc", "Unreachable error occurred: {}", error_message);
                crate::metrics::RPC_UNREACHABLE_ERROR_COUNT
                    .with_label_values(&["RpcProviderError"])
                    .inc();
                Self::InternalError { error_message: error.to_string() }
            }
        }
    }
}
