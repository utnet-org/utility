use std::collections::HashMap;

use unc_primitives::checked_feature;
use unc_primitives::epoch_manager::epoch_info::EpochInfo;
use unc_primitives::epoch_manager::{EpochConfig, RngSeed};
use unc_primitives::epoch_manager::block_info::BlockInfo;
use unc_primitives::epoch_manager::block_summary::BlockSummary;
use unc_primitives::errors::{BlockError, EpochError};
use unc_primitives::hash::CryptoHash;
use unc_primitives::types::validator_power::ValidatorPower;
use unc_primitives::types::validator_stake::ValidatorPledge;
use unc_primitives::types::{AccountId, Balance, NumSeats, ProtocolVersion, ValidatorKickoutReason};

/// Find threshold of pledge per seat, given provided pledges and required number of seats.
pub(crate) fn find_threshold(
    pledges: &[Balance],
    num_seats: NumSeats,
) -> Result<Balance, EpochError> {
    let pledge_sum: Balance = pledges.iter().sum();
    if pledge_sum < num_seats.into() {
        return Err(EpochError::ThresholdError { pledge_sum, num_seats });
    }
    let (mut left, mut right): (Balance, Balance) = (1, pledge_sum + 1);
    'outer: loop {
        if left == right - 1 {
            break Ok(left);
        }
        let mid = (left + right) / 2;
        let mut current_sum: Balance = 0;
        for item in pledges.iter() {
            current_sum += item / mid;
            if current_sum >= u128::from(num_seats) {
                left = mid;
                continue 'outer;
            }
        }
        right = mid;
    }
}
///
pub fn proposals_to_block_summary(
    epoch_config: &EpochConfig,
    this_block_hash: &CryptoHash,
    last_block_hash: &CryptoHash,
    rng_seed: RngSeed,
    prev_block_summary: &BlockInfo,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
    next_version: ProtocolVersion,
) -> Result<BlockSummary, BlockError> {
    return crate::validator_selection::proposals_to_block_summary(
        epoch_config,
        this_block_hash,
        last_block_hash,
        rng_seed,
        prev_block_summary,
        power_proposals,
        pledge_proposals,
        validator_kickout,
        validator_reward,
        minted_amount,
        next_version,
    );
}
/// Calculates new seat assignments based on current seat assignments and proposals.
pub fn proposals_to_epoch_info(
    epoch_config: &EpochConfig,
    rng_seed: RngSeed,
    prev_epoch_info: &EpochInfo,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
    next_version: ProtocolVersion,
    last_epoch_version: ProtocolVersion,
) -> Result<EpochInfo, EpochError> {
    if checked_feature!("stable", AliasValidatorSelectionAlgorithm, last_epoch_version) {
        return crate::validator_selection::proposals_to_epoch_info(
            epoch_config,
            rng_seed,
            prev_epoch_info,
            power_proposals,
            pledge_proposals,
            validator_kickout,
            validator_reward,
            minted_amount,
            next_version,
            last_epoch_version,
        );
    } else {
        return old_validator_selection::proposals_to_epoch_info(
            epoch_config,
            rng_seed,
            prev_epoch_info,
            power_proposals,
            pledge_proposals,
            validator_kickout,
            validator_reward,
            minted_amount,
            next_version,
        );
    }
}

mod old_validator_selection {
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::iter;

    use unc_primitives::epoch_manager::epoch_info::EpochInfo;
    use unc_primitives::epoch_manager::EpochConfig;
    use unc_primitives::errors::EpochError;
    use unc_primitives::types::validator_power::ValidatorPower;
    use unc_primitives::types::validator_stake::ValidatorPledge;
    use unc_primitives::types::{AccountId, Balance, NumSeats, ValidatorPledgeV1, ValidatorId, ValidatorKickoutReason, ValidatorPowerAndPledgeV1, ValidatorPowerV1};
    use unc_primitives::validator_mandates::ValidatorMandates;
    use unc_primitives::version::ProtocolVersion;
    use rand::{RngCore, SeedableRng};
    use rand_hc::Hc128Rng;
    use unc_primitives::types::validator_power_and_pledge::ValidatorPowerAndPledge;

    use crate::proposals::find_threshold;
        use crate::types::RngSeed;

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
    ) -> Result<EpochInfo, EpochError> {
        // Combine proposals with rollovers.
        let mut ordered_power_proposals = BTreeMap::new();
        let mut ordered_pledge_proposals= BTreeMap::new();
        // Account -> new_pledge
        let mut power_change = BTreeMap::new();
        let mut pledge_change = BTreeMap::new();
        let mut fishermen = vec![];
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

        for p in power_proposals {
            let account_id = p.account_id();
            power_change.insert(account_id.clone(), p.power());
            ordered_power_proposals.insert(account_id.clone(), p);
        }

        for f in pledge_proposals {
            let account_id = f.account_id();
            if validator_kickout.contains_key(account_id) {
                let account_id = f.take_account_id();
                pledge_change.insert(account_id,0);
            } else {
                pledge_change.insert(account_id.clone(), f.pledge());
                ordered_pledge_proposals.insert(account_id.clone(), f);
            }
        }

        for r in prev_epoch_info.validators_iter() {
            let account_id = r.account_id().clone();
            if validator_kickout.contains_key(&account_id) {
                pledge_change.insert(account_id,0);
                continue;
            }
            let r_p = ValidatorPower::V1(ValidatorPowerV1{
                account_id: r.account_id().clone(),
                public_key: r.public_key().clone(),
                power: r.power().clone(),
            });
            let p = ordered_power_proposals.entry(account_id.clone()).or_insert(r_p);
            power_change.insert(account_id.clone(), p.power());
            let r_f = ValidatorPledge::V1(ValidatorPledgeV1{
                account_id: r.account_id().clone(),
                public_key: r.public_key().clone(),
                pledge: r.pledge().clone(),
            });
            let f = ordered_pledge_proposals.entry(account_id.clone()).or_insert(r_f);
            //*f.pledge_mut() += *validator_reward.get(&account_id).unwrap_or(&0);
            pledge_change.insert(account_id, f.pledge());
        }

        for r in prev_epoch_info.fishermen_iter() {
            let account_id = r.account_id();
            if validator_kickout.contains_key(account_id) {
                pledge_change.insert(account_id.clone(), 0);
                continue;
            }
            if !ordered_pledge_proposals.contains_key(account_id) {
                // safe to do this here because fishermen from previous epoch is guaranteed to have no
                // duplicates.
                power_change.insert(account_id.clone(), r.power());
                pledge_change.insert(account_id.clone(), r.pledge());
                fishermen.push(r);
            }
        }

        // Get the threshold given current number of seats and pledges.
        let num_hidden_validator_seats: NumSeats =
            epoch_config.avg_hidden_validator_seats_per_shard.iter().sum();
        let num_total_seats = epoch_config.num_block_producer_seats + num_hidden_validator_seats;
        let pledge = ordered_pledge_proposals.iter().map(|(_, p)| p.pledge()).collect::<Vec<_>>();
        let threshold = find_threshold(&pledge, num_total_seats)?;
        // Remove proposals under threshold.
        let mut final_proposals = vec![];

        for (account_id, p) in ordered_pledge_proposals {
            let pledge = p.pledge();
            let power = ordered_power_proposals.get(&account_id.clone()).unwrap().power();
            let p_f = ValidatorPowerAndPledge::V1(ValidatorPowerAndPledgeV1{
                account_id: account_id.clone(),
                public_key: p.public_key().clone(),
                power: power.clone(),
                pledge: pledge.clone(),
            });
            if pledge >= threshold {
                final_proposals.push(p_f);
            } else if pledge >= epoch_config.fishermen_threshold {
                // Do not return pledge back since they will become fishermen
                fishermen.push(p_f);
            } else {
                *pledge_change.get_mut(&account_id).unwrap() = 0;
                if prev_epoch_info.account_is_validator(&account_id)
                    || prev_epoch_info.account_is_fisherman(&account_id)
                {
                    validator_kickout.insert(
                        account_id,
                        ValidatorKickoutReason::NotEnoughPledge { pledge, threshold },
                    );
                }
            }
        }

        // Duplicate each proposal for number of seats it has.
        let mut dup_proposals = final_proposals
            .iter()
            .enumerate()
            .flat_map(|(i, p)| iter::repeat(i as u64).take((p.pledge() / threshold) as usize))
            .collect::<Vec<_>>();

        assert!(dup_proposals.len() >= num_total_seats as usize, "bug in find_threshold");
        shuffle_duplicate_proposals(&mut dup_proposals, rng_seed);

        // Block producers are first `num_block_producer_seats` proposals.
        let mut block_producers_settlement =
            dup_proposals[..epoch_config.num_block_producer_seats as usize].to_vec();
        // remove proposals that are not selected
        let indices_to_keep = block_producers_settlement.iter().copied().collect::<BTreeSet<_>>();
        let (final_proposals, proposals_to_remove) = final_proposals.into_iter().enumerate().fold(
            (vec![], vec![]),
            |(mut proposals, mut to_remove), (i, p)| {
                if indices_to_keep.contains(&(i as u64)) {
                    proposals.push(p);
                } else {
                    to_remove.push(p);
                }
                (proposals, to_remove)
            },
        );
        for p in proposals_to_remove {
            debug_assert!(p.pledge() >= threshold);
            if p.pledge() >= epoch_config.fishermen_threshold {
                fishermen.push(p);
            } else {
                let account_id = p.take_account_id();
                pledge_change.insert(account_id.clone(), 0);
                if prev_epoch_info.account_is_validator(&account_id)
                    || prev_epoch_info.account_is_fisherman(&account_id)
                {
                    validator_kickout.insert(account_id, ValidatorKickoutReason::DidNotGetASeat);
                }
            }
        }

        // reset indices
        for index in block_producers_settlement.iter_mut() {
            *index = indices_to_keep.range(..*index).count() as u64;
        }

        // Collect proposals into block producer assignments.
        let mut chunk_producers_settlement: Vec<Vec<ValidatorId>> = vec![];
        let mut last_index: u64 = 0;
        for num_seats_in_shard in epoch_config.num_block_producer_seats_per_shard.iter() {
            let mut shard_settlement: Vec<ValidatorId> = vec![];
            for _ in 0..*num_seats_in_shard {
                let proposal_index = block_producers_settlement[last_index as usize];
                shard_settlement.push(proposal_index);
                last_index = (last_index + 1) % epoch_config.num_block_producer_seats;
            }
            chunk_producers_settlement.push(shard_settlement);
        }

        let fishermen_to_index = fishermen
            .iter()
            .enumerate()
            .map(|(index, s)| (s.account_id().clone(), index as ValidatorId))
            .collect::<HashMap<_, _>>();

        let validator_to_index = final_proposals
            .iter()
            .enumerate()
            .map(|(index, s)| (s.account_id().clone(), index as ValidatorId))
            .collect::<HashMap<_, _>>();

        // Old validator selection is not aware of chunk validator mandates.
        let validator_mandates: ValidatorMandates = Default::default();
        
        Ok(EpochInfo::new(
            prev_epoch_info.epoch_height() + 1,
            final_proposals,
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

    fn shuffle_duplicate_proposals(dup_proposals: &mut Vec<u64>, rng_seed: RngSeed) {
        let mut rng: Hc128Rng = SeedableRng::from_seed(rng_seed);
        for i in (1..dup_proposals.len()).rev() {
            dup_proposals.swap(i, gen_index_old(&mut rng, (i + 1) as u64) as usize);
        }
    }

    fn gen_index_old(rng: &mut Hc128Rng, bound: u64) -> u64 {
        // This is a simplified copy of the rand gen_index implementation to ensure that
        // upgrades to the rand library will not cause a change in the shuffling behavior.
        let zone = (bound << bound.leading_zeros()).wrapping_sub(1);
        loop {
            let v = rng.next_u64();
            let mul = (v as u128) * (bound as u128);
            let (hi, lo) = ((mul >> 64) as u64, mul as u64);
            if lo < zone {
                return hi;
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use unc_primitives::hash::CryptoHash;

        use crate::proposals::old_validator_selection::shuffle_duplicate_proposals;

        #[test]
        pub fn proposal_shuffling_sanity_checks() {
            // Since we made our own impl for shuffling, do some sanity checks.
            for i in 0..10 {
                let mut dup_proposals = (0..i).collect::<Vec<_>>();
                shuffle_duplicate_proposals(
                    &mut dup_proposals,
                    *CryptoHash::hash_bytes(&[1, 2, 3, 4, 5]).as_bytes(),
                );
                assert_eq!(dup_proposals.len(), i as usize);
                dup_proposals.sort();
                assert_eq!(dup_proposals, (0..i).collect::<Vec<_>>());
            }
        }

        #[test]
        pub fn proposal_randomness_reproducibility() {
            // Sanity check that the proposal shuffling implementation does not change.
            let mut dup_proposals = (0..100).collect::<Vec<u64>>();
            shuffle_duplicate_proposals(
                &mut dup_proposals,
                *CryptoHash::hash_bytes(&[1, 2, 3, 4, 5]).as_bytes(),
            );
            assert_eq!(
                dup_proposals,
                vec![
                    28, 64, 35, 39, 5, 19, 91, 93, 32, 55, 49, 86, 7, 34, 58, 48, 65, 11, 0, 3, 63,
                    85, 96, 12, 23, 76, 29, 69, 31, 45, 1, 15, 33, 61, 38, 74, 87, 10, 62, 9, 40,
                    56, 98, 8, 52, 75, 99, 13, 57, 44, 6, 79, 89, 84, 68, 36, 94, 53, 80, 70, 42,
                    88, 73, 2, 72, 25, 20, 67, 37, 97, 41, 71, 47, 59, 24, 66, 54, 21, 18, 26, 60,
                    92, 50, 77, 81, 14, 43, 17, 90, 95, 78, 16, 30, 46, 22, 83, 27, 4, 51, 82
                ]
            );
        }
    }
}
