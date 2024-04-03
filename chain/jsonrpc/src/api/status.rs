use unc_client_primitives::types::StatusError;
use unc_jsonrpc_primitives::types::status::{
    RpcHealthResponse, RpcStatusError, RpcStatusResponse,
};
use unc_primitives::views::StatusResponse;

use super::RpcFrom;

impl RpcFrom<actix::MailboxError> for RpcStatusError {
    fn rpc_from(error: actix::MailboxError) -> Self {
        Self::InternalError { error_message: error.to_string() }
    }
}

impl RpcFrom<StatusResponse> for RpcStatusResponse {
    fn rpc_from(status_response: StatusResponse) -> Self {
        Self { status_response }
    }
}

impl RpcFrom<unc_client_primitives::debug::DebugStatusResponse>
    for unc_jsonrpc_primitives::types::status::DebugStatusResponse
{
    fn rpc_from(response: unc_client_primitives::debug::DebugStatusResponse) -> Self {
        match response {
            unc_client_primitives::debug::DebugStatusResponse::SyncStatus(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::SyncStatus(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::CatchupStatus(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::CatchupStatus(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::RequestedStateParts(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::RequestedStateParts(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::TrackedShards(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::TrackedShards(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::EpochInfo(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::EpochInfo(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::BlockStatus(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::BlockStatus(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::ValidatorStatus(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::ValidatorStatus(x)
            }
            unc_client_primitives::debug::DebugStatusResponse::ChainProcessingStatus(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::ChainProcessingStatus(
                    x,
                )
            }
        }
    }
}

impl RpcFrom<unc_network::debug::DebugStatus>
    for unc_jsonrpc_primitives::types::status::DebugStatusResponse
{
    fn rpc_from(response: unc_network::debug::DebugStatus) -> Self {
        match response {
            unc_network::debug::DebugStatus::PeerStore(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::PeerStore(x)
            }
            unc_network::debug::DebugStatus::Graph(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::NetworkGraph(x)
            }
            unc_network::debug::DebugStatus::RecentOutboundConnections(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::RecentOutboundConnections(x)
            }
            unc_network::debug::DebugStatus::Routes(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::Routes(x)
            }
            unc_network::debug::DebugStatus::SnapshotHosts(x) => {
                unc_jsonrpc_primitives::types::status::DebugStatusResponse::SnapshotHosts(x)
            }
        }
    }
}

impl RpcFrom<StatusResponse> for RpcHealthResponse {
    fn rpc_from(_status_response: StatusResponse) -> Self {
        Self {}
    }
}

impl RpcFrom<StatusError> for RpcStatusError {
    fn rpc_from(error: StatusError) -> Self {
        match error {
            StatusError::InternalError { error_message } => Self::InternalError { error_message },
            StatusError::NodeIsSyncing => Self::NodeIsSyncing,
            StatusError::NoNewBlocks { elapsed } => Self::NoNewBlocks { elapsed },
            StatusError::EpochOutOfBounds { epoch_id } => Self::EpochOutOfBounds { epoch_id },
            StatusError::Unreachable { ref error_message } => {
                tracing::warn!(target: "jsonrpc", "Unreachable error occurred: {}", error_message);
                crate::metrics::RPC_UNREACHABLE_ERROR_COUNT
                    .with_label_values(&["RpcStatusError"])
                    .inc();
                Self::InternalError { error_message: error.to_string() }
            }
        }
    }
}
