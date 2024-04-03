//! Streamer watches the network and collects all the blocks and related chunks
//! into one struct and pushes in in to the given queue
use std::collections::HashMap;

use actix::Addr;
use futures::stream::StreamExt;
use tracing::warn;

use unc_indexer_primitives::IndexerExecutionOutcomeWithOptionalReceipt;
use unc_o11y::WithSpanContextExt;
use unc_primitives::hash::CryptoHash;
use unc_primitives::{types, views};

use super::errors::FailedToFetchData;
use super::INDEXER;

pub(crate) async fn fetch_status(
    client: &Addr<unc_client::ClientActor>,
) -> Result<unc_primitives::views::StatusResponse, FailedToFetchData> {
    client
        .send(unc_client::Status { is_health_check: false, detailed: false }.with_span_context())
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetches the status to retrieve `latest_block_height` to determine if we need to fetch
/// entire block or we already fetched this block.
pub(crate) async fn fetch_latest_block(
    client: &Addr<unc_client::ViewClientActor>,
) -> Result<views::BlockView, FailedToFetchData> {
    client
        .send(
            unc_client::GetBlock(unc_primitives::types::BlockReference::Finality(
                unc_primitives::types::Finality::Final,
            ))
            .with_span_context(),
        )
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetches specific block by it's height
pub(crate) async fn fetch_block_by_height(
    client: &Addr<unc_client::ViewClientActor>,
    height: u64,
) -> Result<views::BlockView, FailedToFetchData> {
    client
        .send(
            unc_client::GetBlock(unc_primitives::types::BlockId::Height(height).into())
                .with_span_context(),
        )
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetches specific block by it's hash
pub(crate) async fn fetch_block(
    client: &Addr<unc_client::ViewClientActor>,
    hash: CryptoHash,
) -> Result<views::BlockView, FailedToFetchData> {
    client
        .send(
            unc_client::GetBlock(unc_primitives::types::BlockId::Hash(hash).into())
                .with_span_context(),
        )
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

pub(crate) async fn fetch_state_changes(
    client: &Addr<unc_client::ViewClientActor>,
    block_hash: CryptoHash,
    epoch_id: unc_primitives::types::EpochId,
) -> Result<HashMap<unc_primitives::types::ShardId, views::StateChangesView>, FailedToFetchData> {
    client
        .send(
            unc_client::GetStateChangesWithCauseInBlockForTrackedShards { block_hash, epoch_id }
                .with_span_context(),
        )
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetch all ExecutionOutcomeWithId for current block
/// Returns a HashMap where the key is shard id IndexerExecutionOutcomeWithOptionalReceipt
pub(crate) async fn fetch_outcomes(
    client: &Addr<unc_client::ViewClientActor>,
    block_hash: CryptoHash,
) -> Result<
    HashMap<unc_primitives::types::ShardId, Vec<IndexerExecutionOutcomeWithOptionalReceipt>>,
    FailedToFetchData,
> {
    let outcomes = client
        .send(unc_client::GetExecutionOutcomesForBlock { block_hash }.with_span_context())
        .await?
        .map_err(FailedToFetchData::String)?;

    let mut shard_execution_outcomes_with_receipts: HashMap<
        unc_primitives::types::ShardId,
        Vec<IndexerExecutionOutcomeWithOptionalReceipt>,
    > = HashMap::new();
    for (shard_id, shard_outcomes) in outcomes {
        let mut outcomes_with_receipts: Vec<IndexerExecutionOutcomeWithOptionalReceipt> = vec![];
        for outcome in shard_outcomes {
            let receipt = match fetch_receipt_by_id(&client, outcome.id).await {
                Ok(res) => res,
                Err(e) => {
                    warn!(
                        target: INDEXER,
                        "Unable to fetch Receipt with id {}. Skipping it in ExecutionOutcome \n {:#?}",
                        outcome.id,
                        e,
                    );
                    None
                }
            };
            outcomes_with_receipts.push(IndexerExecutionOutcomeWithOptionalReceipt {
                execution_outcome: outcome,
                receipt,
            });
        }
        shard_execution_outcomes_with_receipts.insert(shard_id, outcomes_with_receipts);
    }

    Ok(shard_execution_outcomes_with_receipts)
}

async fn fetch_receipt_by_id(
    client: &Addr<unc_client::ViewClientActor>,
    receipt_id: CryptoHash,
) -> Result<Option<views::ReceiptView>, FailedToFetchData> {
    client
        .send(unc_client::GetReceipt { receipt_id }.with_span_context())
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetches single chunk (as `unc_primitives::views::ChunkView`) by provided
/// chunk hash.
async fn fetch_single_chunk(
    client: &Addr<unc_client::ViewClientActor>,
    chunk_hash: unc_primitives::hash::CryptoHash,
) -> Result<views::ChunkView, FailedToFetchData> {
    client
        .send(unc_client::GetChunk::ChunkHash(chunk_hash.into()).with_span_context())
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))
}

/// Fetches all chunks belonging to given block.
/// Includes transactions and receipts in custom struct (to provide more info).
pub(crate) async fn fetch_block_chunks(
    client: &Addr<unc_client::ViewClientActor>,
    block: &views::BlockView,
) -> Result<Vec<views::ChunkView>, FailedToFetchData> {
    let mut futures: futures::stream::FuturesUnordered<_> = block
        .chunks
        .iter()
        .filter(|chunk| chunk.height_included == block.header.height)
        .map(|chunk| fetch_single_chunk(&client, chunk.chunk_hash))
        .collect();
    let mut chunks = Vec::<views::ChunkView>::with_capacity(futures.len());
    while let Some(chunk) = futures.next().await {
        chunks.push(chunk?);
    }
    Ok(chunks)
}

pub(crate) async fn fetch_protocol_config(
    client: &Addr<unc_client::ViewClientActor>,
    block_hash: unc_primitives::hash::CryptoHash,
) -> Result<unc_chain_configs::ProtocolConfigView, FailedToFetchData> {
    Ok(client
        .send(
            unc_client::GetProtocolConfig(types::BlockReference::from(types::BlockId::Hash(
                block_hash,
            )))
            .with_span_context(),
        )
        .await?
        .map_err(|err| FailedToFetchData::String(err.to_string()))?)
}
