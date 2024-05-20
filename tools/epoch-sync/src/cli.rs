use anyhow::Context;
use clap;
use unc_infra::NightshadeRuntime;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use unc_chain::{ChainStore, ChainStoreAccess, ChainUpdate, DoomslugThresholdMode};
use unc_epoch_manager::shard_tracker::ShardTracker;
use unc_epoch_manager::EpochManager;
use unc_primitives::block::BlockHeader;
use unc_primitives::borsh::BorshDeserialize;
use unc_primitives::epoch_manager::block_info::BlockInfo;
use unc_primitives::epoch_manager::block_summary::{BlockSummary, BlockSummaryV1};
use unc_primitives::epoch_manager::AGGREGATOR_KEY;
use unc_primitives::hash::CryptoHash;
use unc_store::{checkpoint_hot_storage_and_cleanup_columns, DBCol, NodeStorage};

#[derive(clap::Parser)]
pub struct EpochSyncCommand {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(clap::Parser)]
#[clap(subcommand_required = true, arg_required_else_help = true)]
enum SubCommand {
    /// For every finished epoch construct `EpochSyncInfo`
    /// and validate it the same way we would if we received it from a peer.
    ValidateEpochSyncInfo(ValidateEpochSyncInfoCmd),
}

impl EpochSyncCommand {
    pub fn run(self, home_dir: &Path) -> anyhow::Result<()> {
        let mut unc_config = Self::create_snapshot(home_dir)?;
        let storage = unc_infra::open_storage(&home_dir, &mut unc_config)?;

        match self.subcmd {
            SubCommand::ValidateEpochSyncInfo(cmd) => cmd.run(&home_dir, &storage, &unc_config),
        }
    }

    fn create_snapshot(home_dir: &Path) -> anyhow::Result<unc_infra::config::UncConfig> {
        let mut unc_config = unc_infra::config::load_config(
            &home_dir,
            unc_chain_configs::GenesisValidationMode::UnsafeFast,
        )
        .unwrap_or_else(|e| panic!("Error loading config: {e:#}"));

        let store_path_addition = unc_config
            .config
            .store
            .path
            .clone()
            .unwrap_or(PathBuf::from("data"))
            .join("epoch-sync-snapshot");
        let snapshot_path = home_dir.join(store_path_addition.clone());

        let storage = unc_infra::open_storage(&home_dir, &mut unc_config)?;

        if snapshot_path.exists() && snapshot_path.is_dir() {
            tracing::info!(?snapshot_path, "Found a DB snapshot");
        } else {
            tracing::info!(destination = ?snapshot_path, "Creating snapshot of original DB");
            // checkpointing only hot storage, because cold storage will not be changed
            checkpoint_hot_storage_and_cleanup_columns(
                &storage.get_hot_store(),
                &snapshot_path,
                None,
            )?;
        }

        unc_config.config.store.path = Some(store_path_addition.join("data"));

        Ok(unc_config)
    }
}

#[derive(clap::Parser)]
struct ValidateEpochSyncInfoCmd {
    /// If `archive` flag is specified, `BlockInfo` column is assumed to be full and is used for optimisation purposes.
    /// Using `BlockInfo` (`--archive` flag) takes around 10 minutes.
    /// Using `BlockHeader` takes around 1.5 hours.
    #[clap(short, long)]
    archive: bool,
}

impl ValidateEpochSyncInfoCmd {
    pub fn run(
        &self,
        home_dir: &Path,
        storage: &NodeStorage,
        config: &unc_infra::config::UncConfig,
    ) -> anyhow::Result<()> {
        let store = storage.get_hot_store();

        let hash_to_prev_hash = if self.archive {
            get_hash_to_prev_hash_from_block_info(storage)?
        } else {
            get_hash_to_prev_hash_from_block_header(storage)?
        };
        let epoch_ids = get_all_epoch_ids(storage)?;

        let mut chain_store =
            ChainStore::new(store.clone(), config.genesis.config.genesis_height, false);
        let header_head_hash = chain_store.header_head()?.last_block_hash;
        let hash_to_next_hash = get_hash_to_next_hash(&hash_to_prev_hash, &header_head_hash)?;

        let epoch_manager =
            EpochManager::new_arc_handle(storage.get_hot_store(), &config.genesis.config);
        let shard_tracker = ShardTracker::new(epoch_manager.clone());
        let runtime = NightshadeRuntime::from_config(
            home_dir,
            storage.get_hot_store(),
            &config,
            epoch_manager.clone(),
        );
        let chain_update = ChainUpdate::new(
            &mut chain_store,
            epoch_manager,
            shard_tracker,
            runtime,
            DoomslugThresholdMode::TwoThirds,
            config.genesis.config.transaction_validity_period,
        );

        let genesis_hash = store
            .get_ser::<CryptoHash>(
                DBCol::BlockHeight,
                &config.genesis.config.genesis_height.to_le_bytes(),
            )?
            .expect("Expect genesis height to be present in BlockHeight column");

        let mut cur_hash = header_head_hash;

        // Edge case if we exactly at the epoch boundary.
        // In this case we cannot create `EpochSyncInfo` for this epoch yet,
        // as there is no block header with `epoch_sync_data_hash` for that epoch.
        if epoch_ids.contains(&cur_hash) {
            cur_hash = hash_to_prev_hash[&cur_hash];
        }

        let mut num_errors = 0;

        while cur_hash != genesis_hash {
            tracing::debug!(?cur_hash, "Big loop hash");

            // epoch ids are the last hashes of some epochs
            if epoch_ids.contains(&cur_hash) {
                let last_header = store
                    .get_ser::<BlockHeader>(DBCol::BlockHeader, cur_hash.as_ref())?
                    .context("BlockHeader for cur_hash not found")?;
                let last_finalized_height =
                    if *last_header.last_final_block() == CryptoHash::default() {
                        0
                    } else {
                        let last_finalized_header = store
                            .get_ser::<BlockHeader>(
                                DBCol::BlockHeader,
                                last_header.last_final_block().as_ref(),
                            )?
                            .context("BlockHeader for cur_hash.last_final_block not found")?;
                        last_finalized_header.height()
                    };

                loop {
                    let prev_hash = hash_to_prev_hash[&cur_hash];
                    if epoch_ids.contains(&prev_hash) {
                        // prev_hash is the end of previous epoch
                        // cur_hash is the start of current epoch
                        break;
                    } else {
                        // prev_hash is still in the current epoch
                        // we descent to it
                        cur_hash = prev_hash;
                    }
                }

                let first_block_hash = cur_hash;
                let BlockSummary::V1(BlockSummaryV1 {
                    random_value: _random_value,
                    validators,
                    validator_to_index,
                    block_producers_settlement,
                    chunk_producers_settlement,
                    fishermen,
                    fishermen_to_index,
                    power_change,
                    pledge_change,
                    validator_reward,
                    seat_price,
                    minted_amount,
                    all_power_proposals,
                    all_pledge_proposals,
                    validator_kickout,
                    validator_mandates,
                    ..
                }) = BlockSummary::default();
                let mut last_block_info = BlockInfo::new(
                    *last_header.hash(),
                    last_header.height(),
                    last_finalized_height,
                    *last_header.last_final_block(),
                    *last_header.prev_hash(),
                    last_header.prev_validator_power_proposals().collect(),
                    last_header.prev_validator_pledge_proposals().collect(),
                    last_header.chunk_mask().to_vec(),
                    vec![],
                    last_header.total_supply(),
                    last_header.latest_protocol_version(),
                    last_header.raw_timestamp(),
                    // start customized by James Savechives
                    *last_header.random_value(),
                    validators,
                    validator_to_index,
                    block_producers_settlement,
                    chunk_producers_settlement,
                    fishermen,
                    fishermen_to_index,
                    power_change,
                    pledge_change,
                    validator_reward,
                    seat_price,
                    minted_amount,
                    all_power_proposals,
                    all_pledge_proposals,
                    validator_kickout,
                    validator_mandates, // end customized by James Savechives
                );

                *last_block_info.epoch_id_mut() = last_header.epoch_id().clone();
                *last_block_info.epoch_first_block_mut() = first_block_hash;

                let next_epoch_first_hash = hash_to_next_hash[last_header.hash()];
                tracing::debug!("Creating EpochSyncInfo from block {:?}", last_header);

                let epoch_sync_info = chain_update.create_epoch_sync_info(
                    &last_block_info,
                    &next_epoch_first_hash,
                    Some(&hash_to_prev_hash),
                )?;

                let calculated_epoch_sync_data_hash_result =
                    epoch_sync_info.calculate_epoch_sync_data_hash();
                let canonical_epoch_sync_data_hash_result =
                    epoch_sync_info.get_epoch_sync_data_hash();

                if let (Ok(calculated), Ok(Some(canonical))) = (
                    &calculated_epoch_sync_data_hash_result,
                    &canonical_epoch_sync_data_hash_result,
                ) {
                    if calculated == canonical {
                        tracing::info!(
                            "EpochSyncInfo for height {:?} OK",
                            epoch_sync_info.epoch_info.epoch_height()
                        );
                        continue;
                    }
                }
                tracing::error!(
                    "EpochSyncInfo for height {:?} ERROR {:?} {:?}",
                    epoch_sync_info.epoch_info.epoch_height(),
                    calculated_epoch_sync_data_hash_result,
                    canonical_epoch_sync_data_hash_result
                );
                num_errors += 1;
            } else {
                cur_hash = hash_to_prev_hash[&cur_hash];
            }
        }
        assert_eq!(num_errors, 0);
        Ok(())
    }
}

/// Creates mapping from `cur_hash` to `prev_hash` by iterating through `BlockInfo` column.
/// Mapping from `cur_hash` to `prev_hash` is unique, as there are no two blocks with the same hash.
/// This means there will not be key collision. Value collision may happen, but it is irrelevant for `HashMap`.
fn get_hash_to_prev_hash_from_block_info(
    storage: &NodeStorage,
) -> anyhow::Result<HashMap<CryptoHash, CryptoHash>> {
    let mut hash_to_prev_hash = HashMap::new();
    let store = storage.get_split_store().unwrap_or(storage.get_hot_store());
    for result in store.iter(DBCol::BlockInfo) {
        let (_, value) = result?;
        let block_info =
            BlockInfo::try_from_slice(value.as_ref()).expect("Failed to deser BlockInfo");
        if block_info.hash() != block_info.prev_hash() {
            hash_to_prev_hash.insert(*block_info.hash(), *block_info.prev_hash());
        }
    }
    Ok(hash_to_prev_hash)
}

/// Creates mapping from `cur_hash` to `prev_hash` by iterating through `BlockHeader` column.
/// Mapping from `cur_hash` to `prev_hash` is unique, as there are no two blocks with the same hash.
/// This means there will not be key collision. Value collision may happen, but it is irrelevant for `HashMap`.
fn get_hash_to_prev_hash_from_block_header(
    storage: &NodeStorage,
) -> anyhow::Result<HashMap<CryptoHash, CryptoHash>> {
    let mut hash_to_prev_hash = HashMap::new();
    for result in storage.get_hot_store().iter(DBCol::BlockHeader) {
        let (_, value) = result?;
        let block_header =
            BlockHeader::try_from_slice(value.as_ref()).expect("Failed to deser BlockHeader");
        if block_header.hash() != block_header.prev_hash() {
            hash_to_prev_hash.insert(*block_header.hash(), *block_header.prev_hash());
        }
    }
    Ok(hash_to_prev_hash)
}

/// Creates mapping from `cur_hash` to `next_hash` for the chain ending in `chain_head`
/// by descending through mapping from `cur_hash` to `prev_hash`.
/// Only builds mapping for one chain to avoid key collision due to forks.
fn get_hash_to_next_hash(
    hash_to_prev_hash: &HashMap<CryptoHash, CryptoHash>,
    chain_head: &CryptoHash,
) -> anyhow::Result<HashMap<CryptoHash, CryptoHash>> {
    let mut hash_to_next_hash = HashMap::new();
    let mut cur_head = *chain_head;
    while let Some(prev_hash) = hash_to_prev_hash.get(&cur_head) {
        hash_to_next_hash.insert(*prev_hash, cur_head);
        cur_head = *prev_hash;
    }
    Ok(hash_to_next_hash)
}

/// Get all `EpochId`s by iterating `EpochInfo` column and return them as `HashSet<CryptoHash>`.
/// This function is used to get hashes of all last epoch blocks as `EpochId` represents last hash of prev prev column.
fn get_all_epoch_ids(storage: &NodeStorage) -> anyhow::Result<HashSet<CryptoHash>> {
    let mut epoch_ids = HashSet::new();
    for result in storage.get_hot_store().iter(DBCol::EpochInfo) {
        let (key, _) = result?;
        if key.as_ref() == AGGREGATOR_KEY {
            continue;
        }
        epoch_ids
            .insert(CryptoHash::try_from_slice(key.as_ref()).expect("Failed to deser CryptoHash"));
    }
    Ok(epoch_ids)
}
