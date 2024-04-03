use serde_json::Value;
use unc_primitives::types::{AccountId, BlockHeight, EpochId};

#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcProviderError {
    #[error("Block not found")]
    UnknownBlock,
    #[error("Validator info unavailable")]
    ProviderInfoUnavailable,
    #[error("The node reached its limits. Try again later. More details: {error_message}")]
    InternalError { error_message: String },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, arbitrary::Arbitrary, PartialEq, Eq)]
pub struct RpcProviderRequest {
    pub epoch_id: EpochId,
    pub block_height: BlockHeight,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RpcProviderResponse {
    pub provider_account: AccountId,
}

impl From<RpcProviderError> for crate::errors::RpcError {
    fn from(error: RpcProviderError) -> Self {
        let error_data = match &error {
            RpcProviderError::UnknownBlock => Some(Value::String("Unknown Block".to_string())),
            RpcProviderError::ProviderInfoUnavailable => {
                Some(Value::String("Validator info unavailable".to_string()))
            }
            RpcProviderError::InternalError { .. } => Some(Value::String(error.to_string())),
        };

        let error_data_value = match serde_json::to_value(error) {
            Ok(value) => value,
            Err(err) => {
                return Self::new_internal_error(
                    None,
                    format!("Failed to serialize RpcValidatorError: {:?}", err),
                )
            }
        };

        Self::new_internal_or_handler_error(error_data, error_data_value)
    }
}
