use serde_json::Value;
use unc_primitives::types::{CryptoHash, Power};
use unc_primitives::views::validator_power_view::ValidatorPowerView;

#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcAllMinersError {
    #[error("Block not found")]
    UnknownBlock,
    #[error("The node reached its limits. Try again later. More details: {error_message}")]
    InternalError { error_message: String },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, arbitrary::Arbitrary, PartialEq, Eq)]
pub struct RpcAllMinersRequest {
    pub block_hash: CryptoHash,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RpcAllMinersResponse {
    pub total_power: Power,
    pub miners: Vec<ValidatorPowerView>,
}

impl From<RpcAllMinersError> for crate::errors::RpcError {
    fn from(error: RpcAllMinersError) -> Self {
        let error_data = match &error {
            RpcAllMinersError::UnknownBlock => Some(Value::String("Unknown Block".to_string())),
            RpcAllMinersError::InternalError { .. } => Some(Value::String(error.to_string())),
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
