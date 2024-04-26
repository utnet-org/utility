use crate::shard_assignment::assign_shards;
use num_rational::Ratio;
use std::cmp::{self, Ordering};
use std::collections::hash_map;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use unc_primitives::checked_feature;
use unc_primitives::epoch_manager::block_info::BlockInfo;
use unc_primitives::epoch_manager::block_summary::BlockSummary;
use unc_primitives::epoch_manager::epoch_info::EpochInfo;
use unc_primitives::epoch_manager::{EpochConfig, RngSeed};
use unc_primitives::errors::{BlockError, EpochError};
use unc_primitives::hash::CryptoHash;
use unc_primitives::types::validator_power::ValidatorPower;
use unc_primitives::types::validator_power_and_pledge::ValidatorPowerAndPledge;
use unc_primitives::types::validator_stake::ValidatorPledge;
use unc_primitives::types::{
    AccountId, Balance, NumShards, Power, ProtocolVersion, ValidatorId, ValidatorKickoutReason,
    ValidatorPledgeV1, ValidatorPowerAndPledgeV1, ValidatorPowerV1,
};
use unc_primitives::validator_mandates::{ValidatorMandates, ValidatorMandatesConfig};

fn remove_duplicate_power_proposals(power_proposals: Vec<ValidatorPower>) -> Vec<ValidatorPower> {
    let mut unique_proposals_map: HashMap<_, _> = HashMap::new();

    for proposal in power_proposals {
        // Use account_id as the key for uniqueness.
        // This overwrites any existing entry with the same account_id, effectively keeping only the last seen instance.
        unique_proposals_map.insert(proposal.account_id().clone(), proposal);
    }

    // Extract the values (ValidatorPower instances) into a new Vec
    let unique_proposals = unique_proposals_map.into_values().collect::<Vec<_>>();
    unique_proposals
}

fn remove_duplicate_pledge_proposals(
    pledge_proposals: Vec<ValidatorPledge>,
) -> Vec<ValidatorPledge> {
    let mut unique_proposals_map: HashMap<_, _> = HashMap::new();

    for proposal in pledge_proposals {
        // Use account_id as the key for uniqueness.
        // This overwrites any existing entry with the same account_id, effectively keeping only the last seen instance.
        unique_proposals_map.insert(proposal.account_id().clone(), proposal);
    }

    // Extract the values (ValidatorPower instances) into a new Vec
    let unique_proposals = unique_proposals_map.into_values().collect::<Vec<_>>();
    unique_proposals
}
/// Select providers for next block and generate block info
pub fn proposals_to_block_summary(
    epoch_config: &EpochConfig,
    this_block_hash: &CryptoHash,
    last_block_hash: &CryptoHash,
    _rng_seed: RngSeed,
    prev_block_summary: &BlockInfo,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    mut validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
    next_version: ProtocolVersion,
) -> Result<BlockSummary, BlockError> {
    let power_proposals = remove_duplicate_power_proposals(power_proposals);
    let pledge_proposals = remove_duplicate_pledge_proposals(pledge_proposals);
    debug_assert!(
        power_proposals.iter().map(|power| power.account_id()).collect::<HashSet<_>>().len()
            == power_proposals.len(),
        "Power proposals should not have duplicates"
    );

    debug_assert!(
        pledge_proposals.iter().map(|pledge| pledge.account_id()).collect::<HashSet<_>>().len()
            == pledge_proposals.len(),
        "Pledge proposals should not have duplicates"
    );
    let shard_ids: Vec<_> = epoch_config.shard_layout.shard_ids().collect();
    let min_pledge_ratio = {
        let rational = epoch_config.validator_selection_config.minimum_pledge_ratio;
        Ratio::new(*rational.numer() as u128, *rational.denom() as u128)
    };
    let max_bp_selected = epoch_config.num_block_producer_seats as usize;
    let mut power_change = BTreeMap::new();
    let mut pledge_change = BTreeMap::new();
    let mut fishermen = vec![];
    let (power_proposals, pledge_proposals) = proposals_with_rollover_block(
        power_proposals,
        pledge_proposals,
        prev_block_summary,
        &validator_reward,
        &validator_kickout,
        &mut power_change,
        &mut pledge_change,
        &mut fishermen,
    );
    let mut bp_power_proposals = order_power_proposals(power_proposals.values().cloned());
    let mut bp_pledge_proposals = order_pledge_proposals(pledge_proposals.values().cloned());
    let (block_producers, bp_pledge_threshold) = select_block_producers(
        &mut bp_power_proposals,
        &mut bp_pledge_proposals,
        max_bp_selected,
        min_pledge_ratio,
        next_version,
    );
    let (cp_power_proposals, cp_pledge_proposals, chunk_producers, cp_pledge_threshold) =
        if checked_feature!("stable", ChunkOnlyProducers, next_version) {
            let mut cp_power_proposals =
                order_power_proposals(power_proposals.clone().into_values());
            let mut cp_pledge_proposals =
                order_pledge_proposals(pledge_proposals.clone().into_values());
            let max_cp_selected = max_bp_selected
                + (epoch_config.validator_selection_config.num_chunk_only_producer_seats as usize);
            let (chunk_producers, cp_pledge_treshold) = select_chunk_producers(
                &mut cp_power_proposals,
                &mut cp_pledge_proposals,
                max_cp_selected,
                min_pledge_ratio,
                shard_ids.len() as NumShards,
                next_version,
            );
            (cp_power_proposals, cp_pledge_proposals, chunk_producers, cp_pledge_treshold)
        } else {
            (bp_power_proposals, bp_pledge_proposals, block_producers.clone(), bp_pledge_threshold)
        };

    // since block producer proposals could become chunk producers, their actual pledge threshold
    // is the smaller of the two thresholds
    let threshold = cmp::min(bp_pledge_threshold, cp_pledge_threshold);
    let cp_power_map = create_power_map(&cp_power_proposals);
    // process remaining chunk_producer_proposals that were not selected for either role
    for OrderedValidatorPledge(p) in cp_pledge_proposals {
        let pledge = p.pledge();
        let account_id = p.account_id();
        let power = cp_power_map.get(&account_id.clone()).unwrap_or(&0);
        let r_p = ValidatorPowerAndPledge::V1(ValidatorPowerAndPledgeV1 {
            account_id: account_id.clone(),
            public_key: p.public_key().clone(),
            power: *power,
            pledge,
        });
        if pledge >= epoch_config.fishermen_threshold {
            fishermen.push(r_p);
        } else {
            *pledge_change.get_mut(account_id).unwrap() = 0;
            if prev_block_summary.account_is_validator(account_id)
                || prev_block_summary.account_is_fisherman(account_id)
            {
                debug_assert!(pledge < threshold);
                let account_id = p.take_account_id();
                let pledge_value = pledge;
                validator_kickout.insert(
                    account_id,
                    ValidatorKickoutReason::NotEnoughPledge { pledge: pledge_value, threshold },
                );
            }
        }
    }

    let num_chunk_producers = chunk_producers.len();
    // Constructing `all_validators` such that a validators position corresponds to its `ValidatorId`.
    let mut all_validators: Vec<ValidatorPowerAndPledge> = Vec::with_capacity(num_chunk_producers);
    let mut validator_to_index = HashMap::new();
    let mut block_producers_settlement = Vec::with_capacity(block_producers.len());

    for (i, bp) in block_producers.into_iter().enumerate() {
        let id = i as ValidatorId;
        validator_to_index.insert(bp.account_id().clone(), id);
        block_producers_settlement.push(id);
        all_validators.push(bp);
    }

    let chunk_producers_settlement = if checked_feature!("stable", ChunkOnlyProducers, next_version)
    {
        let minimum_validators_per_shard =
            epoch_config.validator_selection_config.minimum_validators_per_shard as usize;
        let shard_assignment = assign_shards(
            chunk_producers,
            shard_ids.len() as NumShards,
            minimum_validators_per_shard,
        )
        .map_err(|_| BlockError::NotEnoughValidators {
            num_validators: num_chunk_producers as u64,
            num_shards: shard_ids.len() as NumShards,
        })?;

        let mut chunk_producers_settlement: Vec<Vec<ValidatorId>> =
            shard_assignment.iter().map(|vs| Vec::with_capacity(vs.len())).collect();
        let mut i = all_validators.len();
        // Here we assign validator ids to all chunk only validators
        for (shard_validators, shard_validator_ids) in
            shard_assignment.into_iter().zip(chunk_producers_settlement.iter_mut())
        {
            for validator in shard_validators {
                debug_assert_eq!(i, all_validators.len());
                match validator_to_index.entry(validator.account_id().clone()) {
                    hash_map::Entry::Vacant(entry) => {
                        let validator_id = i as ValidatorId;
                        entry.insert(validator_id);
                        shard_validator_ids.push(validator_id);
                        all_validators.push(validator);
                        i += 1;
                    }
                    // Validators which have an entry in the validator_to_index map
                    // have already been inserted into `all_validators`.
                    hash_map::Entry::Occupied(entry) => {
                        let validator_id = *entry.get();
                        shard_validator_ids.push(validator_id);
                    }
                }
            }
        }
        chunk_producers_settlement
    } else {
        if chunk_producers.is_empty() {
            // All validators tried to unpledge?
            return Err(BlockError::NotEnoughValidators {
                num_validators: 0u64,
                num_shards: shard_ids.len() as NumShards,
            });
        }
        let mut id = 0usize;
        // Here we assign validators to chunks (we try to keep number of shards assigned for
        // each validator as even as possible). Note that in prod configuration number of seats
        // per shard is the same as maximal number of block producers, so normally all
        // validators would be assigned to all chunks
        shard_ids
            .iter()
            .map(|&shard_id| shard_id as usize)
            .map(|shard_id| {
                (0..epoch_config.num_block_producer_seats_per_shard[shard_id]
                    .min(block_producers_settlement.len() as u64))
                    .map(|_| {
                        let res = block_producers_settlement[id];
                        id = (id + 1) % block_producers_settlement.len();
                        res
                    })
                    .collect()
            })
            .collect()
    };

    let validator_mandates = if checked_feature!("stable", ChunkValidation, next_version) {
        // TODO(#10014) determine required pledge per mandate instead of reusing seat price.
        // TODO(#10014) determine `min_mandates_per_shard`
        let min_mandates_per_shard = 0;
        let validator_mandates_config =
            ValidatorMandatesConfig::new(threshold, min_mandates_per_shard, shard_ids.len());
        // We can use `all_validators` to construct mandates Since a validator's position in
        // `all_validators` corresponds to its `ValidatorId`
        ValidatorMandates::new(validator_mandates_config, &all_validators)
    } else {
        ValidatorMandates::default()
    };

    let fishermen_to_index = fishermen
        .iter()
        .enumerate()
        .map(|(index, s)| (s.account_id().clone(), index as ValidatorId))
        .collect::<HashMap<_, _>>();
    let all_power_proposals = power_proposals.values().cloned().collect();
    let all_pledge_proposals = pledge_proposals.values().cloned().collect();
    Ok(BlockSummary::new(
        *this_block_hash,
        *last_block_hash,
        CryptoHash::default(),
        all_validators,
        validator_to_index,
        block_producers_settlement,
        chunk_producers_settlement,
        fishermen,
        fishermen_to_index,
        power_change,
        pledge_change,
        validator_reward,
        threshold,
        minted_amount,
        all_power_proposals,
        all_pledge_proposals,
        validator_kickout,
        validator_mandates,
    ))
}
/// Select validators for next epoch and generate epoch info
pub fn proposals_to_epoch_info(
    epoch_config: &EpochConfig,
    rng_seed: RngSeed,
    prev_epoch_info: &EpochInfo,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    mut validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
    next_version: ProtocolVersion,
    last_version: ProtocolVersion,
) -> Result<EpochInfo, EpochError> {
    let power_proposals = remove_duplicate_power_proposals(power_proposals);
    let pledge_proposals = remove_duplicate_pledge_proposals(pledge_proposals);
    debug_assert!(
        power_proposals.iter().map(|power| power.account_id()).collect::<HashSet<_>>().len()
            == power_proposals.len(),
        "Power proposals should not have duplicates"
    );

    debug_assert!(
        pledge_proposals.iter().map(|pledge| pledge.account_id()).collect::<HashSet<_>>().len()
            == pledge_proposals.len(),
        "Pledge proposals should not have duplicates"
    );

    let shard_ids: Vec<_> = epoch_config.shard_layout.shard_ids().collect();
    let min_pledge_ratio = {
        let rational = epoch_config.validator_selection_config.minimum_pledge_ratio;
        Ratio::new(*rational.numer() as u128, *rational.denom() as u128)
    };
    let max_bp_selected = epoch_config.num_block_producer_seats as usize;
    let mut power_change = BTreeMap::new();
    let mut pledge_change = BTreeMap::new();
    let mut fishermen = vec![];
    let (power_proposals, pledge_proposals) = proposals_with_rollover(
        power_proposals,
        pledge_proposals,
        prev_epoch_info,
        &validator_reward,
        &validator_kickout,
        &mut power_change,
        &mut pledge_change,
        &mut fishermen,
    );
    let mut bp_power_proposals = order_power_proposals(power_proposals.values().cloned());
    let mut bp_pledge_proposals = order_pledge_proposals(pledge_proposals.values().cloned());
    let (block_producers, bp_pledge_threshold) = select_block_producers(
        &mut bp_power_proposals,
        &mut bp_pledge_proposals,
        max_bp_selected,
        min_pledge_ratio,
        last_version,
    );
    let (cp_power_proposals, cp_pledge_proposals, chunk_producers, cp_pledge_threshold) =
        if checked_feature!("stable", ChunkOnlyProducers, next_version) {
            let mut cp_power_proposals = order_power_proposals(power_proposals.into_values());
            let mut cp_pledge_proposals = order_pledge_proposals(pledge_proposals.into_values());
            let max_cp_selected = max_bp_selected
                + (epoch_config.validator_selection_config.num_chunk_only_producer_seats as usize);
            let (chunk_producers, cp_pledge_treshold) = select_chunk_producers(
                &mut cp_power_proposals,
                &mut cp_pledge_proposals,
                max_cp_selected,
                min_pledge_ratio,
                shard_ids.len() as NumShards,
                last_version,
            );
            (cp_power_proposals, cp_pledge_proposals, chunk_producers, cp_pledge_treshold)
        } else {
            (bp_power_proposals, bp_pledge_proposals, block_producers.clone(), bp_pledge_threshold)
        };

    // since block producer proposals could become chunk producers, their actual pledge threshold
    // is the smaller of the two thresholds
    let threshold = cmp::min(bp_pledge_threshold, cp_pledge_threshold);
    let cp_power_map = create_power_map(&cp_power_proposals);
    // process remaining chunk_producer_proposals that were not selected for either role
    for OrderedValidatorPledge(p) in cp_pledge_proposals {
        let pledge = p.pledge();
        let account_id = p.account_id();
        let power = cp_power_map.get(&account_id.clone()).unwrap_or(&0);
        let r_p = ValidatorPowerAndPledge::V1(ValidatorPowerAndPledgeV1 {
            account_id: account_id.clone(),
            public_key: p.public_key().clone(),
            power: *power,
            pledge,
        });
        if pledge >= epoch_config.fishermen_threshold {
            fishermen.push(r_p);
        } else {
            *pledge_change.get_mut(account_id).unwrap() = 0;
            if prev_epoch_info.account_is_validator(account_id)
                || prev_epoch_info.account_is_fisherman(account_id)
            {
                debug_assert!(pledge < threshold);
                let account_id = p.take_account_id();
                let pledge_value = pledge;
                validator_kickout.insert(
                    account_id,
                    ValidatorKickoutReason::NotEnoughPledge { pledge: pledge_value, threshold },
                );
            }
        }
    }

    let num_chunk_producers = chunk_producers.len();
    // Constructing `all_validators` such that a validators position corresponds to its `ValidatorId`.
    let mut all_validators: Vec<ValidatorPowerAndPledge> = Vec::with_capacity(num_chunk_producers);
    let mut validator_to_index = HashMap::new();
    let mut block_producers_settlement = Vec::with_capacity(block_producers.len());

    for (i, bp) in block_producers.into_iter().enumerate() {
        let id = i as ValidatorId;
        validator_to_index.insert(bp.account_id().clone(), id);
        block_producers_settlement.push(id);
        all_validators.push(bp);
    }

    let chunk_producers_settlement = if checked_feature!("stable", ChunkOnlyProducers, next_version)
    {
        let minimum_validators_per_shard =
            epoch_config.validator_selection_config.minimum_validators_per_shard as usize;
        let shard_assignment = assign_shards(
            chunk_producers,
            shard_ids.len() as NumShards,
            minimum_validators_per_shard,
        )
        .map_err(|_| EpochError::NotEnoughValidators {
            num_validators: num_chunk_producers as u64,
            num_shards: shard_ids.len() as NumShards,
        })?;

        let mut chunk_producers_settlement: Vec<Vec<ValidatorId>> =
            shard_assignment.iter().map(|vs| Vec::with_capacity(vs.len())).collect();
        let mut i = all_validators.len();
        // Here we assign validator ids to all chunk only validators
        for (shard_validators, shard_validator_ids) in
            shard_assignment.into_iter().zip(chunk_producers_settlement.iter_mut())
        {
            for validator in shard_validators {
                debug_assert_eq!(i, all_validators.len());
                match validator_to_index.entry(validator.account_id().clone()) {
                    hash_map::Entry::Vacant(entry) => {
                        let validator_id = i as ValidatorId;
                        entry.insert(validator_id);
                        shard_validator_ids.push(validator_id);
                        all_validators.push(validator);
                        i += 1;
                    }
                    // Validators which have an entry in the validator_to_index map
                    // have already been inserted into `all_validators`.
                    hash_map::Entry::Occupied(entry) => {
                        let validator_id = *entry.get();
                        shard_validator_ids.push(validator_id);
                    }
                }
            }
        }
        chunk_producers_settlement
    } else {
        if chunk_producers.is_empty() {
            // All validators tried to unpledge?
            return Err(EpochError::NotEnoughValidators {
                num_validators: 0u64,
                num_shards: shard_ids.len() as NumShards,
            });
        }
        let mut id = 0usize;
        // Here we assign validators to chunks (we try to keep number of shards assigned for
        // each validator as even as possible). Note that in prod configuration number of seats
        // per shard is the same as maximal number of block producers, so normally all
        // validators would be assigned to all chunks
        shard_ids
            .iter()
            .map(|&shard_id| shard_id as usize)
            .map(|shard_id| {
                (0..epoch_config.num_block_producer_seats_per_shard[shard_id]
                    .min(block_producers_settlement.len() as u64))
                    .map(|_| {
                        let res = block_producers_settlement[id];
                        id = (id + 1) % block_producers_settlement.len();
                        res
                    })
                    .collect()
            })
            .collect()
    };

    let validator_mandates = if checked_feature!("stable", ChunkValidation, next_version) {
        // TODO(#10014) determine required pledge per mandate instead of reusing seat price.
        // TODO(#10014) determine `min_mandates_per_shard`
        let min_mandates_per_shard = 0;
        let validator_mandates_config =
            ValidatorMandatesConfig::new(threshold, min_mandates_per_shard, shard_ids.len());
        // We can use `all_validators` to construct mandates Since a validator's position in
        // `all_validators` corresponds to its `ValidatorId`
        ValidatorMandates::new(validator_mandates_config, &all_validators)
    } else {
        ValidatorMandates::default()
    };

    let fishermen_to_index = fishermen
        .iter()
        .enumerate()
        .map(|(index, s)| (s.account_id().clone(), index as ValidatorId))
        .collect::<HashMap<_, _>>();

    Ok(EpochInfo::new(
        prev_epoch_info.epoch_height() + 1,
        all_validators,
        validator_to_index,
        block_producers_settlement,
        chunk_producers_settlement,
        vec![],
        fishermen,
        fishermen_to_index,
        power_change,
        pledge_change,
        validator_reward,
        validator_kickout,
        minted_amount,
        threshold,
        next_version,
        rng_seed,
        validator_mandates,
    ))
}
/// Copy from proposals_with_rollover
///
///
fn proposals_with_rollover_block(
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    prev_block_summary: &BlockInfo,
    _validator_reward: &HashMap<AccountId, Balance>,
    validator_kickout: &HashMap<AccountId, ValidatorKickoutReason>,
    power_change: &mut BTreeMap<AccountId, Power>,
    pledge_change: &mut BTreeMap<AccountId, Balance>,
    fishermen: &mut Vec<ValidatorPowerAndPledge>,
) -> (HashMap<AccountId, ValidatorPower>, HashMap<AccountId, ValidatorPledge>) {
    let mut power_proposals_by_account = HashMap::new();
    for p in power_proposals {
        let account_id = p.account_id();
        power_change.insert(account_id.clone(), p.power());
        power_proposals_by_account.insert(account_id.clone(), p);
    }

    let mut pledge_proposals_by_account = HashMap::new();
    for f in pledge_proposals {
        let account_id = f.account_id();
        if validator_kickout.contains_key(account_id) {
            let account_id = f.take_account_id();
            pledge_change.insert(account_id, 0);
        } else {
            pledge_change.insert(account_id.clone(), f.pledge());
            pledge_proposals_by_account.insert(account_id.clone(), f);
        }
    }

    for r in prev_block_summary.validators_iter() {
        let account_id = r.account_id().clone();
        if validator_kickout.contains_key(&account_id) {
            pledge_change.insert(account_id, 0);
            continue;
        }
        let r_f = ValidatorPledge::V1(ValidatorPledgeV1 {
            account_id: account_id.clone(),
            public_key: r.public_key().clone(),
            pledge: r.pledge(),
        });
        let r_p = ValidatorPower::V1(ValidatorPowerV1 {
            account_id: account_id.clone(),
            public_key: r.public_key().clone(),
            power: r.power(),
        });
        // The reward will given to the validator in the next epoch,
        // so we need to add it to the balance but not pledge.
        let f = pledge_proposals_by_account.entry(account_id.clone()).or_insert(r_f);
        // if let Some(reward) = validator_reward.get(f.account_id()) {
        //     *f.pledge_mut() += *reward;
        // }
        pledge_change.insert(f.account_id().clone(), f.pledge());

        let p = power_proposals_by_account.entry(account_id).or_insert(r_p);
        power_change.insert(p.account_id().clone(), p.power());
    }

    for r in prev_block_summary.fishermen_iter() {
        let account_id = r.account_id();
        if validator_kickout.contains_key(account_id) {
            pledge_change.insert(account_id.clone(), 0);
            continue;
        }
        if !pledge_proposals_by_account.contains_key(account_id) {
            // safe to do this here because fishermen from previous epoch is guaranteed to have no
            // duplicates.
            power_change.insert(account_id.clone(), r.power());
            pledge_change.insert(account_id.clone(), r.pledge());
            fishermen.push(r);
        }
    }

    (power_proposals_by_account, pledge_proposals_by_account)
}
/// Generates proposals based on new proposals, last epoch validators/fishermen and validator
/// kickouts
/// For each account that was validator or fisherman in last epoch or made pledge action last epoch
/// we apply the following in the order of priority
/// 1. If account is in validator_kickout it cannot be validator or fisherman for the next epoch,
///        we will not include it in proposals or fishermen
/// 2. If account made staking action last epoch, it will be included in proposals with pledge
///        adjusted by rewards from last epoch, if any
/// 3. If account was validator last epoch, it will be included in proposals with the same pledge
///        as last epoch, adjusted by rewards from last epoch, if any
/// 4. If account was fisherman last epoch, it is included in fishermen
fn proposals_with_rollover(
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    prev_epoch_info: &EpochInfo,
    _validator_reward: &HashMap<AccountId, Balance>,
    validator_kickout: &HashMap<AccountId, ValidatorKickoutReason>,
    power_change: &mut BTreeMap<AccountId, Power>,
    pledge_change: &mut BTreeMap<AccountId, Balance>,
    fishermen: &mut Vec<ValidatorPowerAndPledge>,
) -> (HashMap<AccountId, ValidatorPower>, HashMap<AccountId, ValidatorPledge>) {
    let mut power_proposals_by_account = HashMap::new();
    for p in power_proposals {
        let account_id = p.account_id();
        power_change.insert(account_id.clone(), p.power());
        power_proposals_by_account.insert(account_id.clone(), p);
    }

    let mut pledge_proposals_by_account = HashMap::new();
    for f in pledge_proposals {
        let account_id = f.account_id();
        if validator_kickout.contains_key(account_id) {
            let account_id = f.take_account_id();
            pledge_change.insert(account_id, 0);
        } else {
            pledge_change.insert(account_id.clone(), f.pledge());
            pledge_proposals_by_account.insert(account_id.clone(), f);
        }
    }

    for r in prev_epoch_info.validators_iter() {
        let account_id = r.account_id().clone();
        if validator_kickout.contains_key(&account_id) {
            pledge_change.insert(account_id, 0);
            continue;
        }
        let r_f = ValidatorPledge::V1(ValidatorPledgeV1 {
            account_id: account_id.clone(),
            public_key: r.public_key().clone(),
            pledge: r.pledge(),
        });
        let r_p = ValidatorPower::V1(ValidatorPowerV1 {
            account_id: account_id.clone(),
            public_key: r.public_key().clone(),
            power: r.power(),
        });
        // The reward will given to the validator in the next epoch,
        //  so we need to add it to the pledge but not power.
        let f = pledge_proposals_by_account.entry(account_id.clone()).or_insert(r_f);
        // if let Some(reward) = validator_reward.get(f.account_id()) {
        //     *f.pledge_mut() += *reward;
        // }
        pledge_change.insert(f.account_id().clone(), f.pledge());

        let p = power_proposals_by_account.entry(account_id).or_insert(r_p);
        power_change.insert(p.account_id().clone(), p.power());
    }

    for r in prev_epoch_info.fishermen_iter() {
        let account_id = r.account_id();
        if validator_kickout.contains_key(account_id) {
            pledge_change.insert(account_id.clone(), 0);
            continue;
        }
        if !pledge_proposals_by_account.contains_key(account_id) {
            // safe to do this here because fishermen from previous epoch is guaranteed to have no
            // duplicates.
            power_change.insert(account_id.clone(), r.power());
            pledge_change.insert(account_id.clone(), r.pledge());
            fishermen.push(r);
        }
    }

    (power_proposals_by_account, pledge_proposals_by_account)
}

fn order_power_proposals<I: IntoIterator<Item = ValidatorPower>>(
    proposals: I,
) -> BinaryHeap<OrderedValidatorPower> {
    proposals.into_iter().map(OrderedValidatorPower).collect()
}

fn order_pledge_proposals<I: IntoIterator<Item = ValidatorPledge>>(
    proposals: I,
) -> BinaryHeap<OrderedValidatorPledge> {
    proposals.into_iter().map(OrderedValidatorPledge).collect()
}
fn select_block_producers(
    bp_power_proposals: &mut BinaryHeap<OrderedValidatorPower>,
    bp_pledge_proposals: &mut BinaryHeap<OrderedValidatorPledge>,
    max_num_selected: usize,
    min_pledge_ratio: Ratio<u128>,
    protocol_version: ProtocolVersion,
) -> (Vec<ValidatorPowerAndPledge>, Balance) {
    select_validators(
        bp_power_proposals,
        bp_pledge_proposals,
        max_num_selected,
        min_pledge_ratio,
        protocol_version,
    )
}

fn select_chunk_producers(
    all_power_proposals: &mut BinaryHeap<OrderedValidatorPower>,
    all_pledge_proposals: &mut BinaryHeap<OrderedValidatorPledge>,
    max_num_selected: usize,
    min_pledge_ratio: Ratio<u128>,
    num_shards: u64,
    protocol_version: ProtocolVersion,
) -> (Vec<ValidatorPowerAndPledge>, Balance) {
    select_validators(
        all_power_proposals,
        all_pledge_proposals,
        max_num_selected,
        min_pledge_ratio * Ratio::new(1, num_shards as u128),
        protocol_version,
    )
}

#[allow(irrefutable_let_patterns)]
fn create_power_map(heap: &BinaryHeap<OrderedValidatorPower>) -> HashMap<AccountId, Power> {
    let mut power_map = HashMap::new();

    // Explicitly define cloned_heap as an owned BinaryHeap
    let cloned_heap: BinaryHeap<OrderedValidatorPower> = heap.clone();

    // Now we can call into_sorted_vec() on this owned BinaryHeap
    let vec = cloned_heap.into_sorted_vec();

    // Iterate through the vector and populate the hash map
    for ordered_validator in vec {
        if let ValidatorPower::V1(ref v1) = ordered_validator.0 {
            power_map.insert(v1.account_id.clone(), v1.power);
        }
    }

    power_map
}

// Use the function with the appropriate account_id and store
// Takes the top N proposals (by pledge), or fewer if there are not enough or the
// next proposals is too small relative to the others. In the case where all N
// slots are filled, or the pledge ratio falls too low, the threshold pledge to be included
// is also returned.
fn select_validators(
    power_proposals: &mut BinaryHeap<OrderedValidatorPower>,
    pledge_proposals: &mut BinaryHeap<OrderedValidatorPledge>,
    max_number_selected: usize,
    min_pledge_ratio: Ratio<u128>,
    protocol_version: ProtocolVersion,
) -> (Vec<ValidatorPowerAndPledge>, Balance) {
    let mut total_pledge = 0;
    let n = cmp::min(max_number_selected, pledge_proposals.len());
    let mut validators = Vec::with_capacity(n);
    let power_map = create_power_map(power_proposals);
    for _ in 0..n {
        let p = pledge_proposals.pop().unwrap().0;
        let p_pledge = p.pledge();
        let power = power_map.get(&p.account_id().clone()).unwrap_or(&0);
        let p_r = ValidatorPowerAndPledge::V1(ValidatorPowerAndPledgeV1 {
            account_id: p.account_id().clone(),
            public_key: p.public_key().clone(),
            power: *power,
            pledge: p_pledge,
        });
        let total_pledge_with_p = total_pledge + p_pledge;
        if total_pledge_with_p != 0 && Ratio::new(p_pledge, total_pledge_with_p) > min_pledge_ratio
        {
            validators.push(p_r);
            total_pledge = total_pledge_with_p;
        } else {
            // p was not included, return it to the list of proposals
            pledge_proposals.push(OrderedValidatorPledge(p));
            break;
        }
    }
    if validators.len() == max_number_selected {
        // all slots were filled, so the threshold pledge is 1 more than the current
        // smallest pledge
        let threshold = validators.last().unwrap().pledge() + 1;
        (validators, threshold)
    } else {
        // the pledge ratio condition prevented all slots from being filled,
        // or there were fewer proposals than available slots,
        // so the threshold pledge is whatever amount pass the pledge ratio condition
        let threshold = if checked_feature!(
            "protocol_feature_fix_staking_threshold",
            FixStakingThreshold,
            protocol_version
        ) {
            (min_pledge_ratio * Ratio::from_integer(total_pledge)
                / (Ratio::from_integer(1u128) - min_pledge_ratio))
                .ceil()
                .to_integer()
        } else {
            (min_pledge_ratio * Ratio::new(total_pledge, 1)).ceil().to_integer()
        };
        (validators, threshold)
    }
}

/// We store pledges in max heap and want to order them such that the validator
/// with the largest state and (in case of a tie) lexicographically smallest
/// AccountId comes at the top.
#[derive(Eq, PartialEq, Clone)]
struct OrderedValidatorPledge(ValidatorPledge);
impl OrderedValidatorPledge {
    fn key(&self) -> impl Ord + '_ {
        (self.0.pledge(), std::cmp::Reverse(self.0.account_id()))
    }
}
impl PartialOrd for OrderedValidatorPledge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedValidatorPledge {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}
#[derive(Eq, PartialEq, Clone)]
struct OrderedValidatorPower(ValidatorPower);
impl OrderedValidatorPower {
    fn key(&self) -> impl Ord + '_ {
        (self.0.power(), std::cmp::Reverse(self.0.account_id()))
    }
}
impl PartialOrd for OrderedValidatorPower {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedValidatorPower {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_rational::Ratio;
    use unc_crypto::{KeyType, PublicKey};
    use unc_primitives::account::id::AccountIdRef;
    use unc_primitives::epoch_manager::epoch_info::{EpochInfo, EpochInfoV3};
    use unc_primitives::epoch_manager::ValidatorSelectionConfig;
    use unc_primitives::shard_layout::ShardLayout;
    use unc_primitives::types::validator_power::ValidatorPower;
    #[cfg(feature = "nightly")]
    use unc_primitives::validator_mandates::{AssignmentWeight, ValidatorMandatesAssignment};
    use unc_primitives::version::PROTOCOL_VERSION;

    #[test]
    fn test_validator_assignment_all_block_producers() {
        // A simple sanity test. Given fewer proposals than the number of seats,
        // none of which has too little pledge, they all get assigned as block and
        // chunk producers.
        let epoch_config = create_epoch_config(2, 100, 0, Default::default());
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &["test1", "test2"], &[]);
        let power_proposals =
            create_power_proposals(&[("test1", 1000), ("test2", 2000), ("test3", 300)]);
        let pledge_proposals =
            create_pledge_proposals(&[("test1", 1000), ("test2", 2000), ("test3", 300)]);
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals.clone(),
            pledge_proposals.clone(),
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        // increment height
        assert_eq!(epoch_info.epoch_height(), prev_epoch_height + 1);

        // assign block producers in decreasing order of pledge
        let mut sorted_proposals = power_proposals;
        sorted_proposals.sort_unstable_by(|a, b| b.power().cmp(&a.power()));
        assert_eq!(sorted_proposals, epoch_info.validators_iter().collect::<Vec<_>>());

        // All proposals become block producers
        assert_eq!(epoch_info.block_producers_settlement(), &[0, 1, 2]);
        assert_eq!(epoch_info.fishermen_iter().len(), 0);

        // Validators are split between chunks to make roughly equal pledges
        // (in this case shard 0 has 2000, while shard 1 has 1300).
        assert_eq!(epoch_info.chunk_producers_settlement(), &[vec![0], vec![1, 2]]);
    }

    #[test]
    fn test_validator_assignment_with_chunk_only_producers() {
        // A more complex test. Here there are more BP proposals than spots, so some will
        // become chunk-only producers, along side the other chunk-only proposals.
        let num_bp_seats = 10;
        let num_cp_seats = 30;
        let epoch_config = create_epoch_config(
            2,
            num_bp_seats,
            // purposely set the fishermen threshold high so that none become fishermen
            10_000,
            ValidatorSelectionConfig {
                num_chunk_only_producer_seats: num_cp_seats,
                minimum_validators_per_shard: 1,
                minimum_pledge_ratio: Ratio::new(160, 1_000_000),
            },
        );
        let prev_epoch_height = 3;
        let test1_pledge = 1000;
        let prev_epoch_info = create_prev_epoch_info(
            prev_epoch_height,
            &[
                // test1 is not included in proposals below, and will get kicked out because
                // their pledge is too low.
                ("test1", test1_pledge, Proposal::BlockProducer),
                // test2 submitted a new proposal, so their pledge will come from there, but it
                // too will be kicked out
                ("test2", 1234, Proposal::ChunkOnlyProducer),
            ],
            &[],
        );
        let pledge_proposals =
            create_pledge_proposals((2..(2 * num_bp_seats + num_cp_seats)).map(|i| {
                (
                    format!("test{}", i),
                    2000u128 + (i as u128),
                    if i <= num_cp_seats {
                        Proposal::ChunkOnlyProducer
                    } else {
                        Proposal::BlockProducer
                    },
                )
            }));
        let power_proposals =
            create_power_proposals((2..(2 * num_bp_seats + num_cp_seats)).map(|i| {
                (
                    format!("test{}", i),
                    2000u128 + (i as u128),
                    if i <= num_cp_seats {
                        Proposal::ChunkOnlyProducer
                    } else {
                        Proposal::BlockProducer
                    },
                )
            }));
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals.clone(),
            pledge_proposals.clone(),
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        assert_eq!(epoch_info.epoch_height(), prev_epoch_height + 1);

        // the top pledges are the chosen block producers
        let mut sorted_proposals = pledge_proposals;
        sorted_proposals.sort_unstable_by(|a, b| b.power().cmp(&a.power()));
        assert!(sorted_proposals.iter().take(num_bp_seats as usize).cloned().eq(epoch_info
            .block_producers_settlement()
            .into_iter()
            .map(|id| epoch_info.get_validator(*id))));

        // pledges are evenly distributed between shards
        assert_eq!(
            pledge_sum(&epoch_info, epoch_info.chunk_producers_settlement()[0].iter()),
            pledge_sum(&epoch_info, epoch_info.chunk_producers_settlement()[1].iter()),
        );

        // The top proposals are all chunk producers
        let mut chosen_chunk_producers: Vec<ValidatorPower> = epoch_info
            .chunk_producers_settlement()
            .into_iter()
            .flatten()
            .map(|id| epoch_info.get_validator(*id))
            .collect();
        chosen_chunk_producers.sort_unstable_by(|a, b| b.power().cmp(&a.power()));
        assert!(sorted_proposals
            .into_iter()
            .take((num_bp_seats + num_cp_seats) as usize)
            .eq(chosen_chunk_producers));

        // the old, low-pledge proposals were not accepted
        let kickout = epoch_info.validator_kickout();
        assert_eq!(kickout.len(), 2);
        assert_eq!(
            kickout.get(AccountIdRef::new_or_panic("test1")).unwrap(),
            &ValidatorKickoutReason::NotEnoughPledge { pledge: test1_pledge, threshold: 2011 },
        );
        assert_eq!(
            kickout.get(AccountIdRef::new_or_panic("test2")).unwrap(),
            &ValidatorKickoutReason::NotEnoughPledge { pledge: 2002, threshold: 2011 },
        );
    }

    #[test]
    fn test_block_producer_sampling() {
        let num_shards = 4;
        let epoch_config = create_epoch_config(
            num_shards,
            2,
            0,
            ValidatorSelectionConfig {
                num_chunk_only_producer_seats: 0,
                minimum_validators_per_shard: 1,
                minimum_pledge_ratio: Ratio::new(160, 1_000_000),
            },
        );
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &["test1", "test2"], &[]);
        let power_proposals = create_power_proposals(&[("test1", 1000), ("test2", 2000)]);
        let pledge_proposals = create_pledge_proposals(&[("test1", 1000), ("test2", 2000)]);

        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        // test2 is chosen as the bp 2x more often than test1
        let mut counts: [i32; 2] = [0, 0];
        for h in 0..100_000 {
            let bp = epoch_info.sample_block_producer(h);
            counts[bp as usize] += 1;
        }
        let diff = (2 * counts[1] - counts[0]).abs();
        assert!(diff < 100);
    }

    #[test]
    fn test_chunk_producer_sampling() {
        // When there is 1 CP per shard, they are chosen 100% of the time.
        let num_shards = 4;
        let epoch_config = create_epoch_config(
            num_shards,
            2 * num_shards,
            0,
            ValidatorSelectionConfig {
                num_chunk_only_producer_seats: 0,
                minimum_validators_per_shard: 1,
                minimum_pledge_ratio: Ratio::new(160, 1_000_000),
            },
        );
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &["test1", "test2"], &[]);
        let power_proposals = create_power_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000),
            ("test4", 1000),
        ]);
        let pledge_proposals = create_pledge_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000),
            ("test4", 1000),
        ]);

        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        for shard_id in 0..num_shards {
            for h in 0..100_000 {
                let cp = epoch_info.sample_chunk_producer(h, shard_id);
                // Don't read too much into this. The reason the ValidatorId always
                // equals the ShardId is because the validators are assigned to shards in order.
                assert_eq!(cp, shard_id)
            }
        }

        // When there are multiple CPs they are chosen in proportion to pledge.
        let power_proposals = create_power_proposals((1..=num_shards).flat_map(|i| {
            // Each shard gets a pair of validators, one with twice as
            // much pledge as the other.
            vec![(format!("test{}", i), 1000), (format!("test{}", 100 * i), 2000)].into_iter()
        }));
        let pledge_proposals = create_pledge_proposals((1..=num_shards).flat_map(|i| {
            // Each shard gets a pair of validators, one with twice as
            // much pledge as the other.
            vec![(format!("test{}", i), 1000), (format!("test{}", 100 * i), 2000)].into_iter()
        }));
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        for shard_id in 0..num_shards {
            let mut counts: [i32; 2] = [0, 0];
            for h in 0..100_000 {
                let cp = epoch_info.sample_chunk_producer(h, shard_id);
                // if ValidatorId is in the second half then it is the lower
                // pledge validator (because they are sorted by decreasing pledge).
                let index = if cp >= num_shards { 1 } else { 0 };
                counts[index] += 1;
            }
            let diff = (2 * counts[1] - counts[0]).abs();
            assert!(diff < 500);
        }
    }

    /// This test only verifies that chunk validator mandates are correctly wired up with
    /// `EpochInfo`. The internals of mandate assignment are tested in the module containing
    /// [`ValidatorMandates`].
    #[test]
    #[cfg(feature = "nightly")]
    fn test_chunk_validators_sampling() {
        let num_shards = 4;
        let epoch_config = create_epoch_config(
            num_shards,
            2 * num_shards,
            0,
            ValidatorSelectionConfig {
                num_chunk_only_producer_seats: 0,
                minimum_validators_per_shard: 1,
                // for example purposes, we choose a higher ratio than in production
                minimum_pledge_ratio: Ratio::new(1, 10),
            },
        );
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &["test1", "test2"], &[]);

        // Choosing proposals s.t. the `threshold` (i.e. seat price) calculated in
        // `proposals_to_epoch_info` below will be 100. For now, this `threshold` is used as the
        // pledge required for a chunk validator mandate.
        //
        // Note that `proposals_to_epoch_info` will not include `test6` in the set of validators,
        // hence it will not hold a (partial) mandate
        let power_proposals = create_power_proposals(&[
            ("test1", 1500),
            ("test2", 1000),
            ("test3", 1000),
            ("test4", 260),
            ("test5", 140),
            ("test6", 50),
        ]);
        let pledge_proposals = create_pledge_proposals(&[
            ("test1", 1500),
            ("test2", 1000),
            ("test3", 1000),
            ("test4", 260),
            ("test5", 140),
            ("test6", 50),
        ]);

        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        // Given `epoch_info` and `proposals` above, the sample at a given height is deterministic.
        let height = 42;
        let expected_assignments: ValidatorMandatesAssignment = vec![
            HashMap::from([
                (0, AssignmentWeight::new(3, 0)),
                (1, AssignmentWeight::new(3, 0)),
                (2, AssignmentWeight::new(3, 0)),
                (3, AssignmentWeight::new(0, 60)),
            ]),
            HashMap::from([
                (0, AssignmentWeight::new(6, 0)),
                (1, AssignmentWeight::new(2, 0)),
                (2, AssignmentWeight::new(2, 0)),
            ]),
            HashMap::from([
                (0, AssignmentWeight::new(4, 0)),
                (1, AssignmentWeight::new(1, 0)),
                (2, AssignmentWeight::new(3, 0)),
                (3, AssignmentWeight::new(2, 0)),
            ]),
            HashMap::from([
                (0, AssignmentWeight::new(2, 0)),
                (1, AssignmentWeight::new(4, 0)),
                (2, AssignmentWeight::new(2, 0)),
                (4, AssignmentWeight::new(1, 40)),
            ]),
        ];
        assert_eq!(epoch_info.sample_chunk_validators(height), expected_assignments);
    }

    #[test]
    fn test_validator_assignment_ratio_condition() {
        // There are more seats than proposals, however the
        // lower proposals are too small relative to the total
        // (the reason we can't choose them is because the the probability of them actually
        // being selected to make a block would be too low since it is done in
        // proportion to pledge).
        let epoch_config = create_epoch_config(
            1,
            100,
            150,
            ValidatorSelectionConfig {
                num_chunk_only_producer_seats: 300,
                minimum_validators_per_shard: 1,
                // for example purposes, we choose a higher ratio than in production
                minimum_pledge_ratio: Ratio::new(1, 10),
            },
        );
        let prev_epoch_height = 7;
        // test5 and test6 are going to get kicked out for not enough pledge.
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &["test5", "test6"], &[]);
        let power_proposals = create_power_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
        ]);
        let pledge_proposals = create_pledge_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
        ]);
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        // pledge below validator threshold, but above fishermen threshold become fishermen
        let fishermen: Vec<_> = epoch_info.fishermen_iter().map(|v| v.take_account_id()).collect();
        assert_eq!(fishermen, vec!["test4"]);

        // too low pledges are kicked out
        let kickout = epoch_info.validator_kickout();
        assert_eq!(kickout.len(), 2);
        #[cfg(feature = "protocol_feature_fix_staking_threshold")]
        let expected_threshold = 334;
        #[cfg(not(feature = "protocol_feature_fix_staking_threshold"))]
        let expected_threshold = 300;
        assert_eq!(
            kickout.get(AccountIdRef::new_or_panic("test5")).unwrap(),
            &ValidatorKickoutReason::NotEnoughPower { power: 100, threshold: expected_threshold },
        );
        assert_eq!(
            kickout.get(AccountIdRef::new_or_panic("test6")).unwrap(),
            &ValidatorKickoutReason::NotEnoughPower { power: 50, threshold: expected_threshold },
        );

        let bp_threshold = epoch_info.seat_price();
        let num_validators = epoch_info.validators_iter().len();
        let power_proposals = create_power_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
            ("test7", bp_threshold),
        ]);
        let pledge_proposals = create_pledge_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
            ("test7", bp_threshold),
        ]);
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();
        #[cfg(feature = "protocol_feature_fix_staking_threshold")]
        assert_eq!(num_validators + 1, epoch_info.validators_iter().len());

        let power_proposals = create_power_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
            ("test7", bp_threshold - 1),
        ]);
        let pledge_proposals = create_pledge_proposals(&[
            ("test1", 1000),
            ("test2", 1000),
            ("test3", 1000), // the total up to this point is 3000
            ("test4", 200),  // 200 is < 1/10 of 3000, so not validator, but can be fisherman
            ("test5", 100),  // 100 is even too small to be a fisherman, cannot get any role
            ("test6", 50),
            ("test7", bp_threshold - 1),
        ]);
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &epoch_info,
            power_proposals,
            pledge_proposals,
            Default::default(),
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();
        assert_eq!(num_validators, epoch_info.validators_iter().len());
    }

    #[test]
    fn test_validator_assignment_with_kickout() {
        // kicked out validators are not selected
        let epoch_config = create_epoch_config(1, 100, 0, Default::default());
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(
            prev_epoch_height,
            &[("test1", 10_000), ("test2", 2000), ("test3", 3000)],
            &[],
        );
        let mut kick_out = HashMap::new();
        // test1 is kicked out
        kick_out.insert("test1".parse().unwrap(), ValidatorKickoutReason::Unpowered);
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            Default::default(),
            Default::default(),
            kick_out,
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        // test1 is not selected
        assert_eq!(epoch_info.get_validator_id(&"test1".parse().unwrap()), None);
    }

    #[test]
    fn test_validator_assignment_with_rewards() {
        // validator balances are updated based on their rewards
        let validators = [("test1", 3000), ("test2", 2000), ("test3", 1000)];
        let rewards: [u128; 3] = [7, 8, 9];
        let epoch_config = create_epoch_config(1, 100, 0, Default::default());
        let prev_epoch_height = 7;
        let prev_epoch_info = create_prev_epoch_info(prev_epoch_height, &validators, &[]);
        let rewards_map = validators
            .iter()
            .zip(rewards.iter())
            .map(|((name, _), reward)| (name.parse().unwrap(), *reward))
            .collect();
        let epoch_info = proposals_to_epoch_info(
            &epoch_config,
            [0; 32],
            &prev_epoch_info,
            Default::default(),
            Default::default(),
            rewards_map,
            Default::default(),
            0,
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .unwrap();

        for (v, ((_, power), reward)) in
            epoch_info.validators_iter().zip(validators.iter().zip(rewards.iter()))
        {
            assert_eq!(v.power(), power + reward);
        }
    }

    fn pledge_sum<'a, I: IntoIterator<Item = &'a u64>>(
        epoch_info: &EpochInfo,
        validator_ids: I,
    ) -> u128 {
        validator_ids.into_iter().map(|id| epoch_info.get_validator(*id).power()).sum()
    }

    /// Create EpochConfig, only filling in the fields important for validator selection.
    fn create_epoch_config(
        num_shards: u64,
        num_block_producer_seats: u64,
        fishermen_threshold: Balance,
        validator_selection_config: ValidatorSelectionConfig,
    ) -> EpochConfig {
        EpochConfig {
            epoch_length: 10,
            num_block_producer_seats,
            num_block_producer_seats_per_shard: vec![num_block_producer_seats; num_shards as usize],
            avg_hidden_validator_seats_per_shard: vec![0; num_shards as usize],
            block_producer_kickout_threshold: 0,
            chunk_producer_kickout_threshold: 0,
            validator_max_kickout_pledge_perc: 100,
            online_min_threshold: 0.into(),
            online_max_threshold: 0.into(),
            fishermen_threshold,
            minimum_pledge_divisor: 0,
            protocol_upgrade_pledge_threshold: 0.into(),
            shard_layout: ShardLayout::v0(num_shards, 0),
            validator_selection_config,
        }
    }

    fn create_prev_epoch_info<T: IntoValidatorPledge + Copy>(
        epoch_height: u64,
        prev_validators: &[T],
        prev_fishermen: &[T],
    ) -> EpochInfo {
        let mut result: EpochInfoV3 = Default::default();

        result.epoch_height = epoch_height;
        result.validators = create_proposals(prev_validators);
        result.fishermen = create_proposals(prev_fishermen);

        result.validator_to_index = to_map(&result.validators);
        result.fishermen_to_index = to_map(&result.fishermen);

        EpochInfo::V3(result)
    }

    fn to_map(vs: &[ValidatorPowerAndPledge]) -> HashMap<AccountId, ValidatorId> {
        vs.iter().enumerate().map(|(i, v)| (v.account_id().clone(), i as u64)).collect()
    }

    fn create_proposals<I, T>(ps: I) -> Vec<ValidatorPowerAndPledge>
    where
        T: IntoValidatorPledge,
        I: IntoIterator<Item = T>,
    {
        ps.into_iter().map(IntoValidatorPledge::into_validator_pledge).collect()
    }
    fn create_power_proposals<I, T>(ps: I) -> Vec<ValidatorPower>
    where
        T: IntoValidatorPledge,
        I: IntoIterator<Item = T>,
    {
        ps.into_iter().map(IntoValidatorPledge::into_validator_pledge).collect()
    }

    fn create_pledge_proposals<I, T>(ps: I) -> Vec<ValidatorPledge>
    where
        T: IntoValidatorPledge,
        I: IntoIterator<Item = T>,
    {
        ps.into_iter().map(IntoValidatorPledge::into_validator_pledge).collect()
    }

    #[derive(Debug, PartialEq, Eq, Copy, Clone)]
    enum Proposal {
        BlockProducer,
        ChunkOnlyProducer,
    }

    trait IntoValidatorPledge {
        fn into_validator_pledge(self) -> ValidatorPledge;
    }

    impl IntoValidatorPledge for &str {
        fn into_validator_pledge(self) -> ValidatorPledge {
            ValidatorPledge::new(self.parse().unwrap(), PublicKey::empty(KeyType::ED25519), 100)
        }
    }

    impl IntoValidatorPledge for (&str, Balance) {
        fn into_validator_pledge(self) -> ValidatorPledge {
            ValidatorPledge::new(
                self.0.parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                self.1,
            )
        }
    }

    impl IntoValidatorPledge for (String, Balance) {
        fn into_validator_pledge(self) -> ValidatorPledge {
            ValidatorPledge::new(
                self.0.parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                self.1,
            )
        }
    }

    impl IntoValidatorPledge for (&str, Balance, Proposal) {
        fn into_validator_pledge(self) -> ValidatorPledge {
            ValidatorPledge::new(
                self.0.parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                self.1,
            )
        }
    }

    impl IntoValidatorPledge for (String, Balance, Proposal) {
        fn into_validator_pledge(self) -> ValidatorPledge {
            ValidatorPledge::new(
                self.0.parse().unwrap(),
                PublicKey::empty(KeyType::ED25519),
                self.1,
            )
        }
    }

    impl<T: IntoValidatorPledge + Copy> IntoValidatorPledge for &T {
        fn into_validator_pledge(self) -> ValidatorPledge {
            (*self).into_validator_pledge()
        }
    }
}
