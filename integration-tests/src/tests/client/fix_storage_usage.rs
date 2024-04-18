use borsh::BorshDeserialize;
use framework::config::GenesisExt;
use framework::test_utils::TestEnvNightshadeSetupExt;
use unc_chain::{ChainGenesis, Provenance};
use unc_chain_configs::Genesis;
use unc_client::test_utils::TestEnv;
use unc_o11y::testonly::init_test_logger;
use unc_primitives::version::ProtocolFeature;
use unc_primitives::{trie_key::TrieKey, types::AccountId};
use unc_store::{ShardUId, TrieUpdate};

use crate::tests::client::process_blocks::set_block_protocol_version;

fn process_blocks_with_storage_usage_fix(
    chain_id: String,
    check_storage_usage: fn(AccountId, u64, u64),
) {
    let epoch_length = 5;
    let mut genesis = Genesis::test(vec!["test0".parse().unwrap()], 1);
    genesis.config.chain_id = chain_id;
    genesis.config.epoch_length = epoch_length;
    genesis.config.protocol_version = ProtocolFeature::FixStorageUsage.protocol_version() - 1;
    let chain_genesis = ChainGenesis::new(&genesis);
    let mut env = TestEnv::builder(chain_genesis)
        .real_epoch_managers(&genesis.config)
        .nightshade_runtimes(&genesis)
        .build();
    for i in 1..=16 {
        // We cannot just use TestEnv::produce_block as we are updating protocol version
        let mut block = env.clients[0].produce_block(i).unwrap().unwrap();
        set_block_protocol_version(
            &mut block,
            "test0".parse().unwrap(),
            ProtocolFeature::FixStorageUsage.protocol_version(),
        );

        let _ = env.clients[0].process_block_test(block.clone().into(), Provenance::NONE).unwrap();
        env.clients[0].finish_blocks_in_processing();

        let root = *env.clients[0]
            .chain
            .get_chunk_extra(block.hash(), &ShardUId::single_shard())
            .unwrap()
            .state_root();
        let trie =
            env.clients[0].runtime_adapter.get_trie_for_shard(0, block.hash(), root, true).unwrap();
        let state_update = TrieUpdate::new(trie);
        use unc_primitives::account::Account;
        let mut account_unc_raw = state_update
            .get(&TrieKey::Account { account_id: "unc".parse().unwrap() })
            .unwrap()
            .unwrap()
            .clone();
        let account_unc = Account::try_from_slice(&mut account_unc_raw).unwrap();
        let mut account_test0_raw = state_update
            .get(&TrieKey::Account { account_id: "test0".parse().unwrap() })
            .unwrap()
            .unwrap()
            .clone();
        let account_test0 = Account::try_from_slice(&mut account_test0_raw).unwrap();
        check_storage_usage("unc".parse().unwrap(), i, account_unc.storage_usage());
        check_storage_usage("test0".parse().unwrap(), i, account_test0.storage_usage());
    }
}

#[test]
fn test_fix_storage_usage_migration() {
    init_test_logger();
    process_blocks_with_storage_usage_fix(
        unc_primitives::chains::MAINNET.to_string(),
        |account_id: AccountId, block_height: u64, storage_usage: u64| {
            if account_id == "unc" && block_height >= 11 {
                assert_eq!(storage_usage, 4378);
            } else {
                assert_eq!(storage_usage, 182);
            }
        },
    );
    process_blocks_with_storage_usage_fix(
        unc_primitives::chains::TESTNET.to_string(),
        |_: AccountId, _: u64, storage_usage: u64| {
            assert_eq!(storage_usage, 182);
        },
    );
}
