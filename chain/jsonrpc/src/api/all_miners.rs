use serde_json::Value;

use unc_client_primitives::types::GetAllMinersError;
use unc_jsonrpc_primitives::errors::RpcParseError;
use unc_jsonrpc_primitives::types::all_miners::{RpcAllMinersError, RpcAllMinersRequest};
use unc_primitives::hash::CryptoHash;

use super::{RpcFrom, RpcRequest};

impl RpcRequest for RpcAllMinersRequest {
    fn parse(value: Value) -> Result<Self, RpcParseError> {
        // Extract block_hash_str from value
        let block_hash_str = value.get("block_hash").and_then(|v| v.as_str()).ok_or_else(|| {
            RpcParseError("block_hash not found or not a string".parse().unwrap())
        })?;

        // Decode the base58-encoded string to bytes
        let bytes = bs58::decode(block_hash_str)
            .into_vec()
            .map_err(|_| RpcParseError("Invalid base58-encoded hash".parse().unwrap()))?;

        // Ensure the decoded bytes have the correct length for a CryptoHash
        if bytes.len() != 32 {
            return Err(RpcParseError(
                "Decoded hash does not match expected length".parse().unwrap(),
            ));
        }

        // Construct the CryptoHash from the decoded bytes
        let block_hash: CryptoHash = block_hash_str.parse().map_err(|_| {
            RpcParseError("Failed to parse block_hash from base58".parse().unwrap())
        })?;

        Ok(Self { block_hash })
    }
}

impl RpcFrom<actix::MailboxError> for RpcAllMinersError {
    fn rpc_from(error: actix::MailboxError) -> Self {
        Self::InternalError { error_message: error.to_string() }
    }
}

impl RpcFrom<GetAllMinersError> for RpcAllMinersError {
    fn rpc_from(error: GetAllMinersError) -> Self {
        match error {
            GetAllMinersError::UnknownBlock { .. } => Self::UnknownBlock {},
            GetAllMinersError::IOError { error_message } => Self::InternalError { error_message },
            GetAllMinersError::Unreachable { ref error_message } => {
                tracing::warn!(target: "jsonrpc", "Unreachable error occurred: {}", error_message);
                crate::metrics::RPC_UNREACHABLE_ERROR_COUNT
                    .with_label_values(&["RpcProviderError"])
                    .inc();
                Self::InternalError { error_message: error.to_string() }
            }
        }
    }
}
