use std::sync::Arc;

use crate::EpochManagerAdapter;
use unc_primitives::hash::CryptoHash;
use unc_primitives::types::{AccountId, ShardId};

/// TrackedConfig::AllShards: track all shards
#[derive(Clone)]
pub struct ShardTracker {
    epoch_manager: Arc<dyn EpochManagerAdapter>,
}

impl ShardTracker {
    pub fn new(epoch_manager: Arc<dyn EpochManagerAdapter>) -> Self {
        ShardTracker { epoch_manager }
    }

    pub fn new_empty(epoch_manager: Arc<dyn EpochManagerAdapter>) -> Self {
        Self::new(epoch_manager)
    }

    /// Whether the client cares about some shard right now.
    /// * If `account_id` is None, `is_me` is not checked and the
    /// result indicates whether the client is tracking the shard
    /// * If `account_id` is not None, it is supposed to be a validator
    /// account and `is_me` indicates whether we check what shards
    /// the client tracks.
    pub fn care_about_shard(
        &self,
        account_id: Option<&AccountId>,
        parent_hash: &CryptoHash,
        shard_id: ShardId,
        is_me: bool,
    ) -> bool {
        // TODO: fix these unwrap_or here and handle error correctly. The current behavior masks potential errors and bugs
        // https://github.com/utnet-org/utility/issues/4936
        if let Some(account_id) = account_id {
            let account_cares_about_shard = self
                .epoch_manager
                .cares_about_shard_from_prev_block(parent_hash, account_id, shard_id)
                .unwrap_or(false);
            if account_cares_about_shard {
                // An account has to track this shard because of its validation duties.
                return true;
            }
            if !is_me {
                // We don't know how another node is configured.
                // It may track all shards, it may track no additional shards.
                return false;
            } else {
                // We have access to the node config. Use the config to find a definite answer.
            }
        }
        true
    }

    /// Whether the client cares about some shard in the next epoch.
    ///  Note that `shard_id` always refers to a shard in the current epoch
    ///  If shard layout will change next epoch,
    ///  returns true if it cares about any shard that `shard_id` will split to
    /// * If `account_id` is None, `is_me` is not checked and the
    /// result indicates whether the client will track the shard
    /// * If `account_id` is not None, it is supposed to be a validator
    /// account and `is_me` indicates whether we check what shards
    /// the client will track.
    pub fn will_care_about_shard(
        &self,
        account_id: Option<&AccountId>,
        parent_hash: &CryptoHash,
        shard_id: ShardId,
        is_me: bool,
    ) -> bool {
        if let Some(account_id) = account_id {
            let account_cares_about_shard = {
                self.epoch_manager
                    .cares_about_shard_next_epoch_from_prev_block(parent_hash, account_id, shard_id)
                    .unwrap_or(false)
            };
            if account_cares_about_shard {
                // An account has to track this shard because of its validation duties.
                return true;
            }
            if !is_me {
                // We don't know how another node is configured.
                // It may track all shards, it may track no additional shards.
                return false;
            } else {
                // We have access to the node config. Use the config to find a definite answer.
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::ShardTracker;
    use crate::{EpochManager, EpochManagerHandle, RewardCalculator};
    use num_rational::Ratio;
    use std::collections::HashSet;
    use std::sync::Arc;
    use unc_crypto::{KeyType, PublicKey};
    use unc_primitives::epoch_manager::block_info::BlockInfo;
    use unc_primitives::epoch_manager::{AllEpochConfig, EpochConfig};
    use unc_primitives::hash::CryptoHash;
    use unc_primitives::shard_layout::ShardLayout;
    use unc_primitives::types::validator_power::ValidatorPower;
    use unc_primitives::types::validator_stake::ValidatorPledge;
    use unc_primitives::types::{BlockHeight, NumShards, ProtocolVersion, ShardId};
    use unc_primitives::version::PROTOCOL_VERSION;
    use unc_store::test_utils::create_test_store;

    const DEFAULT_TOTAL_SUPPLY: u128 = 1_000_000_000_000;

    fn get_epoch_manager(
        genesis_protocol_version: ProtocolVersion,
        num_shards: NumShards,
        use_production_config: bool,
    ) -> EpochManagerHandle {
        let store = create_test_store();
        let initial_epoch_config = EpochConfig {
            epoch_length: 1,
            num_block_producer_seats: 1,
            num_block_producer_seats_per_shard: vec![1],
            avg_hidden_validator_seats_per_shard: vec![],
            block_producer_kickout_threshold: 90,
            chunk_producer_kickout_threshold: 60,
            fishermen_threshold: 0,
            online_max_threshold: Ratio::from_integer(1),
            online_min_threshold: Ratio::new(90, 100),
            minimum_pledge_divisor: 1,
            protocol_upgrade_pledge_threshold: Ratio::new(80, 100),
            shard_layout: ShardLayout::v0(num_shards, 0),
            validator_selection_config: Default::default(),
            validator_max_kickout_pledge_perc: 100,
        };
        let reward_calculator = RewardCalculator {
            max_inflation_rate: Ratio::from_integer(0),
            num_blocks_per_year: 1000000,
            epoch_length: 1,
            protocol_reward_rate: Ratio::from_integer(0),
            protocol_treasury_account: "test".parse().unwrap(),
            online_max_threshold: initial_epoch_config.online_max_threshold,
            online_min_threshold: initial_epoch_config.online_min_threshold,
            num_seconds_per_year: 1000000,
        };
        EpochManager::new(
            store,
            AllEpochConfig::new(use_production_config, initial_epoch_config, "test-chain"),
            genesis_protocol_version,
            reward_calculator,
            vec![ValidatorPower::new(
                "test".parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                100,
            )],
            vec![ValidatorPledge::new(
                "test".parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                100,
            )],
        )
        .unwrap()
        .into_handle()
    }

    pub fn record_block(
        epoch_manager: &mut EpochManager,
        prev_h: CryptoHash,
        cur_h: CryptoHash,
        height: BlockHeight,
        power_proposals: Vec<ValidatorPower>,
        pledge_proposals: Vec<ValidatorPledge>,
        protocol_version: ProtocolVersion,
    ) {
        epoch_manager
            .record_block_info(
                BlockInfo::new(
                    cur_h,
                    height,
                    0,
                    prev_h,
                    prev_h,
                    power_proposals,
                    pledge_proposals,
                    vec![],
                    vec![],
                    DEFAULT_TOTAL_SUPPLY,
                    protocol_version,
                    height * 10u64.pow(9),
                    ..Default::default(),
                ),
                [0; 32],
            )
            .unwrap()
            .commit()
            .unwrap();
    }

    fn get_all_shards_care_about(
        tracker: &ShardTracker,
        shard_ids: &[ShardId],
        parent_hash: &CryptoHash,
    ) -> HashSet<ShardId> {
        shard_ids
            .into_iter()
            .filter(|&&shard_id| tracker.care_about_shard(None, parent_hash, shard_id, true))
            .cloned()
            .collect()
    }

    fn get_all_shards_will_care_about(
        tracker: &ShardTracker,
        shard_ids: &[ShardId],
        parent_hash: &CryptoHash,
    ) -> HashSet<ShardId> {
        shard_ids
            .into_iter()
            .filter(|&&shard_id| tracker.will_care_about_shard(None, parent_hash, shard_id, true))
            .cloned()
            .collect()
    }

    #[test]
    fn test_track_all_shards() {
        let shard_ids: Vec<_> = (0..4).collect();
        let epoch_manager =
            get_epoch_manager(PROTOCOL_VERSION, shard_ids.len() as NumShards, false);
        let tracker = ShardTracker::new(Arc::new(epoch_manager));
        let total_tracked_shards: HashSet<_> = shard_ids.iter().cloned().collect();

        assert_eq!(
            get_all_shards_care_about(&tracker, &shard_ids, &CryptoHash::default()),
            total_tracked_shards
        );
        assert_eq!(
            get_all_shards_will_care_about(&tracker, &shard_ids, &CryptoHash::default()),
            total_tracked_shards
        );
    }
}
