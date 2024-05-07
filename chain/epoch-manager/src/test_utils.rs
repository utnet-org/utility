use std::collections::{BTreeMap, HashMap};

use num_rational::Ratio;
use unc_primitives::types::{EpochId, Power};
use unc_store::Store;

use crate::proposals::find_threshold;
use crate::RewardCalculator;
use crate::RngSeed;
use crate::{BlockInfo, EpochManager};
use unc_crypto::{KeyType, SecretKey};
use unc_primitives::challenge::SlashedValidator;
use unc_primitives::epoch_manager::block_info::BlockInfoV2;
use unc_primitives::epoch_manager::epoch_info::EpochInfo;
use unc_primitives::epoch_manager::{AllEpochConfig, EpochConfig, ValidatorWeight};
use unc_primitives::hash::{hash, CryptoHash};
use unc_primitives::types::validator_power::ValidatorPower;
use unc_primitives::types::{
    AccountId, Balance, BlockHeight, BlockHeightDelta, EpochHeight, NumSeats, NumShards,
    ValidatorId, ValidatorKickoutReason,
};
use unc_primitives::utils::get_num_seats_per_shard;
use unc_primitives::validator_mandates::{ValidatorMandates, ValidatorMandatesConfig};
use unc_primitives::version::PROTOCOL_VERSION;
use unc_store::test_utils::create_test_store;

use unc_primitives::shard_layout::ShardLayout;
use unc_primitives::types::validator_power_and_pledge::ValidatorPowerAndPledge;
use unc_primitives::types::validator_stake::ValidatorPledge;
use {crate::reward_calculator::NUM_NS_IN_SECOND, crate::NUM_SECONDS_IN_A_YEAR};

pub const DEFAULT_GAS_PRICE: u128 = 100;
pub const DEFAULT_TOTAL_SUPPLY: u128 = 1_000_000_000_000;
pub const TEST_SEED: RngSeed = [3; 32];

pub fn hash_range(num: usize) -> Vec<CryptoHash> {
    let mut result = vec![];
    for i in 0..num {
        result.push(hash(i.to_le_bytes().as_ref()));
    }
    result
}

pub fn epoch_info(
    epoch_height: EpochHeight,
    accounts: Vec<(AccountId, Power, Balance)>,
    block_producers_settlement: Vec<ValidatorId>,
    chunk_producers_settlement: Vec<Vec<ValidatorId>>,
    hidden_validators_settlement: Vec<ValidatorWeight>,
    fishermen: Vec<(AccountId, Power, Balance)>,
    power_change: BTreeMap<AccountId, Power>,
    pledge_change: BTreeMap<AccountId, Balance>,
    validator_kickout: Vec<(AccountId, ValidatorKickoutReason)>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
) -> EpochInfo {
    let num_seats = block_producers_settlement.len() as u64;
    epoch_info_with_num_seats(
        epoch_height,
        accounts,
        block_producers_settlement,
        chunk_producers_settlement,
        hidden_validators_settlement,
        fishermen,
        power_change,
        pledge_change,
        validator_kickout,
        validator_reward,
        minted_amount,
        num_seats,
    )
}

pub fn epoch_info_with_num_seats(
    epoch_height: EpochHeight,
    mut accounts: Vec<(AccountId, Power, Balance)>,
    block_producers_settlement: Vec<ValidatorId>,
    chunk_producers_settlement: Vec<Vec<ValidatorId>>,
    hidden_validators_settlement: Vec<ValidatorWeight>,
    fishermen: Vec<(AccountId, Power, Balance)>,
    power_change: BTreeMap<AccountId, Power>,
    pledge_change: BTreeMap<AccountId, Balance>,
    validator_kickout: Vec<(AccountId, ValidatorKickoutReason)>,
    validator_reward: HashMap<AccountId, Balance>,
    minted_amount: Balance,
    num_seats: NumSeats,
) -> EpochInfo {
    let seat_price =
        find_threshold(&accounts.iter().map(|(_, _, s)| *s).collect::<Vec<_>>(), num_seats)
            .unwrap();
    accounts.sort();
    let validator_to_index = accounts.iter().enumerate().fold(HashMap::new(), |mut acc, (i, x)| {
        acc.insert(x.0.clone(), i as u64);
        acc
    });
    let fishermen_to_index =
        fishermen.iter().enumerate().map(|(i, (s, _, _))| (s.clone(), i as ValidatorId)).collect();
    let account_to_validators =
        |accounts: Vec<(AccountId, Power, Balance)>| -> Vec<ValidatorPowerAndPledge> {
            accounts
                .into_iter()
                .map(|(account_id, power, pledging)| {
                    ValidatorPowerAndPledge::new(
                        account_id.clone(),
                        SecretKey::from_seed(KeyType::ED25519, account_id.as_ref()).public_key(),
                        power,
                        pledging,
                    )
                })
                .collect()
        };
    let all_validators = account_to_validators(accounts);
    let validator_mandates = {
        // TODO(#10014) determine required pledge per mandate instead of reusing seat price.
        // TODO(#10014) determine `min_mandates_per_shard`
        let num_shards = chunk_producers_settlement.len();
        let min_mandates_per_shard = 0;
        let config = ValidatorMandatesConfig::new(seat_price, min_mandates_per_shard, num_shards);
        ValidatorMandates::new(config, &all_validators)
    };
    EpochInfo::new(
        epoch_height,
        all_validators,
        validator_to_index,
        block_producers_settlement,
        chunk_producers_settlement,
        hidden_validators_settlement,
        account_to_validators(fishermen),
        fishermen_to_index,
        power_change,
        pledge_change,
        validator_reward,
        validator_kickout.into_iter().collect(),
        minted_amount,
        seat_price,
        PROTOCOL_VERSION,
        TEST_SEED,
        validator_mandates,
    )
}

pub fn epoch_config_with_production_config(
    epoch_length: BlockHeightDelta,
    num_shards: NumShards,
    num_block_producer_seats: NumSeats,
    num_hidden_validator_seats: NumSeats,
    block_producer_kickout_threshold: u8,
    chunk_producer_kickout_threshold: u8,
    fishermen_threshold: Balance,
    use_production_config: bool,
) -> AllEpochConfig {
    let epoch_config = EpochConfig {
        epoch_length,
        num_block_producer_seats,
        num_block_producer_seats_per_shard: get_num_seats_per_shard(
            num_shards,
            num_block_producer_seats,
        ),
        avg_hidden_validator_seats_per_shard: (0..num_shards)
            .map(|_| num_hidden_validator_seats)
            .collect(),
        block_producer_kickout_threshold,
        chunk_producer_kickout_threshold,
        fishermen_threshold,
        online_min_threshold: Ratio::new(90, 100),
        online_max_threshold: Ratio::new(99, 100),
        protocol_upgrade_pledge_threshold: Ratio::new(80, 100),
        minimum_pledge_divisor: 1,
        validator_selection_config: Default::default(),
        shard_layout: ShardLayout::v0(num_shards, 0),
        validator_max_kickout_pledge_perc: 100,
    };
    AllEpochConfig::new(use_production_config, epoch_config, "test-chain")
}

pub fn epoch_config(
    epoch_length: BlockHeightDelta,
    num_shards: NumShards,
    num_block_producer_seats: NumSeats,
    num_hidden_validator_seats: NumSeats,
    block_producer_kickout_threshold: u8,
    chunk_producer_kickout_threshold: u8,
    fishermen_threshold: Balance,
) -> AllEpochConfig {
    epoch_config_with_production_config(
        epoch_length,
        num_shards,
        num_block_producer_seats,
        num_hidden_validator_seats,
        block_producer_kickout_threshold,
        chunk_producer_kickout_threshold,
        fishermen_threshold,
        false,
    )
}

pub fn power(account_id: AccountId, power: Power) -> ValidatorPower {
    let public_key = SecretKey::from_seed(KeyType::ED25519, account_id.as_ref()).public_key();
    ValidatorPower::new(account_id, public_key, power)
}

pub fn pledge(account_id: AccountId, pledge: Balance) -> ValidatorPledge {
    let public_key = SecretKey::from_seed(KeyType::ED25519, account_id.as_ref()).public_key();
    ValidatorPledge::new(account_id, public_key, pledge)
}

/// No-op reward calculator. Will produce no reward
pub fn default_reward_calculator() -> RewardCalculator {
    RewardCalculator {
        max_inflation_rate: Ratio::from_integer(0),
        num_blocks_per_year: 1,
        epoch_length: 1,
        protocol_reward_rate: Ratio::from_integer(0),
        protocol_treasury_account: "unc".parse().unwrap(),
        online_min_threshold: Ratio::new(90, 100),
        online_max_threshold: Ratio::new(99, 100),
        num_seconds_per_year: NUM_SECONDS_IN_A_YEAR,
    }
}

pub fn reward(info: Vec<(AccountId, Balance)>) -> HashMap<AccountId, Balance> {
    info.into_iter().collect()
}

pub fn setup_epoch_manager(
    power_validators: Vec<(AccountId, Power)>,
    pledge_validators: Vec<(AccountId, Balance)>,
    epoch_length: BlockHeightDelta,
    num_shards: NumShards,
    num_block_producer_seats: NumSeats,
    num_hidden_validator_seats: NumSeats,
    block_producer_kickout_threshold: u8,
    chunk_producer_kickout_threshold: u8,
    fishermen_threshold: Balance,
    reward_calculator: RewardCalculator,
) -> EpochManager {
    let store = create_test_store();
    let config = epoch_config(
        epoch_length,
        num_shards,
        num_block_producer_seats,
        num_hidden_validator_seats,
        block_producer_kickout_threshold,
        chunk_producer_kickout_threshold,
        fishermen_threshold,
    );
    EpochManager::new(
        store,
        config,
        PROTOCOL_VERSION,
        reward_calculator,
        power_validators
            .iter()
            .map(|(account_id, power)| power(account_id.clone(), *power))
            .collect(),
        pledge_validators
            .iter()
            .map(|(account_id, balance)| pledge(account_id.clone(), *balance))
            .collect(),
    )
    .unwrap()
}

pub fn setup_default_epoch_manager(
    power_validators: Vec<(AccountId, Power)>,
    pledge_validators: Vec<(AccountId, Balance)>,
    epoch_length: BlockHeightDelta,
    num_shards: NumShards,
    num_block_producer_seats: NumSeats,
    num_hidden_validator_seats: NumSeats,
    block_producer_kickout_threshold: u8,
    chunk_producer_kickout_threshold: u8,
) -> EpochManager {
    setup_epoch_manager(
        power_validators,
        pledge_validators,
        epoch_length,
        num_shards,
        num_block_producer_seats,
        num_hidden_validator_seats,
        block_producer_kickout_threshold,
        chunk_producer_kickout_threshold,
        1,
        default_reward_calculator(),
    )
}

/// Makes an EpochManager with the given block and chunk producers,
/// automatically coming up with pledges for them to ensure the desired
/// election outcome.
pub fn setup_epoch_manager_with_block_and_chunk_producers(
    store: Store,
    block_producers: Vec<AccountId>,
    chunk_only_producers: Vec<AccountId>,
    num_shards: NumShards,
    epoch_length: BlockHeightDelta,
) -> EpochManager {
    let num_block_producers = block_producers.len() as u64;
    let block_producer_power = 1_000_000 as u64;
    let block_producer_pledge = 1_000_000 as u128;
    let mut total_pledge = 0;
    let mut total_power = 0;
    let mut power_validators = vec![];
    let mut pledge_validators = vec![];
    for block_producer in &block_producers {
        power_validators.push((block_producer.clone(), block_producer_power));
        pledge_validators.push((block_producer.clone(), block_producer_pledge));
        total_pledge += block_producer_pledge;
        total_power += block_producer_power;
    }
    for chunk_only_producer in &chunk_only_producers {
        let minimum_pledge_to_ensure_election =
            total_pledge * 160 / 1_000_000 / num_shards as u128 + 1;
        let pledge = block_producer_pledge - 1;
        assert!(
            pledge >= minimum_pledge_to_ensure_election,
            "Could not honor the specified list of producers"
        );
        let minimum_power_to_ensure_election =
            total_power * 160 / 1_000_000 / num_shards as u64 + 1;
        let power = block_producer_power - 1;
        assert!(
            power >= minimum_power_to_ensure_election,
            "Could not honor the specified list of producers"
        );
        power_validators.push((chunk_only_producer.clone(), power));
        pledge_validators.push((chunk_only_producer.clone(), pledge));
        total_pledge += pledge;
        total_power += power;
    }
    let config = epoch_config(epoch_length, num_shards, num_block_producers, 0, 0, 0, 0);
    let epoch_manager = EpochManager::new(
        store,
        config,
        PROTOCOL_VERSION,
        default_reward_calculator(),
        power_validators
            .iter()
            .map(|(account_id, weight)| power(account_id.clone(), *weight))
            .collect(),
        pledge_validators
            .iter()
            .map(|(account_id, balance)| pledge(account_id.clone(), *balance))
            .collect(),
    )
    .unwrap();
    // Sanity check that the election results are indeed as expected.
    let actual_block_producers = epoch_manager
        .get_all_block_producers_ordered(&EpochId::default(), &CryptoHash::default())
        .unwrap();
    assert_eq!(actual_block_producers.len(), block_producers.len());
    let actual_chunk_producers =
        epoch_manager.get_all_chunk_producers(&EpochId::default()).unwrap();
    assert_eq!(actual_chunk_producers.len(), block_producers.len() + chunk_only_producers.len());
    epoch_manager
}

pub fn record_block_with_final_block_hash(
    epoch_manager: &mut EpochManager,
    prev_h: CryptoHash,
    cur_h: CryptoHash,
    last_final_block_hash: CryptoHash,
    height: BlockHeight,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
) {
    epoch_manager
        .record_block_info(
            BlockInfo::new(
                cur_h,
                height,
                height.saturating_sub(2),
                last_final_block_hash,
                prev_h,
                power_proposals.clone(),
                pledge_proposals.clone(),
                vec![],
                vec![],
                DEFAULT_TOTAL_SUPPLY,
                PROTOCOL_VERSION,
                height * NUM_NS_IN_SECOND,
                CryptoHash::default(),
                vec![],
                HashMap::default(),
                vec![],
                vec![],
                vec![],
                HashMap::default(),
                BTreeMap::default(),
                BTreeMap::default(),
                HashMap::default(),
                0,
                0,
                power_proposals,
                pledge_proposals,
                HashMap::default(),
                ValidatorMandates::default(),
            ),
            [0; 32],
        )
        .unwrap()
        .commit()
        .unwrap();
}

pub fn record_block_with_slashes(
    epoch_manager: &mut EpochManager,
    prev_h: CryptoHash,
    cur_h: CryptoHash,
    height: BlockHeight,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
    slashed: Vec<SlashedValidator>,
) {
    epoch_manager
        .record_block_info(
            BlockInfo::new(
                cur_h,
                height,
                height.saturating_sub(2),
                prev_h,
                prev_h,
                power_proposals.clone(),
                pledge_proposals.clone(),
                vec![],
                slashed,
                DEFAULT_TOTAL_SUPPLY,
                PROTOCOL_VERSION,
                height * NUM_NS_IN_SECOND,
                CryptoHash::default(),
                vec![],
                HashMap::default(),
                vec![],
                vec![],
                vec![],
                HashMap::default(),
                BTreeMap::default(),
                BTreeMap::default(),
                HashMap::default(),
                0,
                0,
                power_proposals,
                pledge_proposals,
                HashMap::default(),
                ValidatorMandates::default(),
            ),
            [0; 32],
        )
        .unwrap()
        .commit()
        .unwrap();
}

pub fn record_block(
    epoch_manager: &mut EpochManager,
    prev_h: CryptoHash,
    cur_h: CryptoHash,
    height: BlockHeight,
    power_proposals: Vec<ValidatorPower>,
    pledge_proposals: Vec<ValidatorPledge>,
) {
    record_block_with_slashes(
        epoch_manager,
        prev_h,
        cur_h,
        height,
        power_proposals,
        pledge_proposals,
        vec![],
    );
}

pub fn block_info(
    hash: CryptoHash,
    height: BlockHeight,
    last_finalized_height: BlockHeight,
    last_final_block_hash: CryptoHash,
    prev_hash: CryptoHash,
    epoch_first_block: CryptoHash,
    chunk_mask: Vec<bool>,
    total_supply: Balance,
    random_value: CryptoHash,
    validators: Vec<ValidatorPowerAndPledge>,
    validator_to_index: HashMap<AccountId, ValidatorId>,
    block_producers_settlement: Vec<ValidatorId>,
    chunk_producers_settlement: Vec<Vec<ValidatorId>>,
    fishermen: Vec<ValidatorPowerAndPledge>,
    fishermen_to_index: HashMap<AccountId, ValidatorId>,
    power_change: BTreeMap<AccountId, Power>,
    pledge_change: BTreeMap<AccountId, Balance>,
    validator_reward: HashMap<AccountId, Balance>,
    seat_price: Balance,
    minted_amount: Balance,
    all_power_proposals: Vec<ValidatorPower>,
    all_pledge_proposals: Vec<ValidatorPledge>,
    validator_kickout: HashMap<AccountId, ValidatorKickoutReason>,
    validator_mandates: ValidatorMandates,
) -> BlockInfo {
    BlockInfo::V2(BlockInfoV2 {
        hash,
        height,
        last_finalized_height,
        last_final_block_hash,
        prev_hash,
        epoch_first_block,
        epoch_id: Default::default(),
        power_proposals: vec![],
        pledge_proposals: vec![],
        chunk_mask,
        latest_protocol_version: PROTOCOL_VERSION,
        slashed: Default::default(),
        total_supply,
        timestamp_nanosec: height * NUM_NS_IN_SECOND,
        random_value,
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
    })
}

pub fn record_with_block_info(epoch_manager: &mut EpochManager, block_info: BlockInfo) {
    epoch_manager.record_block_info(block_info, [0; 32]).unwrap().commit().unwrap();
}
