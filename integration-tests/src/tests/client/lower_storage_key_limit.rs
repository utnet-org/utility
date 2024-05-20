use assert_matches::assert_matches;
use unc_chain::{ChainGenesis, Provenance};
use unc_chain_configs::Genesis;
use unc_client::test_utils::TestEnv;
use unc_client::ProcessTxResponse;
use unc_crypto::{InMemorySigner, KeyType, Signer};
use unc_infra::config::GenesisExt;
use unc_infra::test_utils::TestEnvNightshadeSetupExt;
use unc_o11y::testonly::init_test_logger;
use unc_parameters::RuntimeConfigStore;
use unc_primitives::errors::TxExecutionError;
use unc_primitives::hash::CryptoHash;
use unc_primitives::transaction::{Action, FunctionCallAction, Transaction};
use unc_primitives::types::BlockHeight;
use unc_primitives::version::PROTOCOL_VERSION;
use unc_primitives::views::FinalExecutionStatus;

use crate::tests::client::process_blocks::deploy_test_contract_with_protocol_version;

/// Check correctness of the protocol upgrade and ability to write 2 KB keys.
#[test]
fn protocol_upgrade() {
    init_test_logger();

    let new_protocol_version = PROTOCOL_VERSION;
    let old_protocol_version = new_protocol_version - 1;
    let new_storage_key_limit = 2usize.pow(11); // 2 KB
    let args: Vec<u8> = vec![1u8; new_storage_key_limit + 1]
        .into_iter()
        .chain(unc_primitives::test_utils::encode(&[10u64]).into_iter())
        .collect();
    let epoch_length: BlockHeight = 5;

    // The immediate protocol upgrade needs to be set for this test to pass in
    // the release branch where the protocol upgrade date is set.
    std::env::set_var("UNC_TESTS_IMMEDIATE_PROTOCOL_UPGRADE", "1");

    // Prepare TestEnv with a contract at the old protocol version.
    let mut env = {
        let mut genesis =
            Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
        genesis.config.epoch_length = epoch_length;
        genesis.config.protocol_version = old_protocol_version;
        let chain_genesis = ChainGenesis::new(&genesis);
        let mut env = TestEnv::builder(chain_genesis)
            .real_epoch_managers(&genesis.config)
            .track_all_shards()
            .nightshade_runtimes_with_runtime_config_store(
                &genesis,
                vec![RuntimeConfigStore::new(None)],
            )
            .build();

        deploy_test_contract_with_protocol_version(
            &mut env,
            "test0".parse().unwrap(),
            unc_test_contracts::rs_contract(),
            epoch_length,
            1,
            old_protocol_version,
        );
        env
    };

    let signer = InMemorySigner::from_seed("test0".parse().unwrap(), KeyType::ED25519, "test0");
    let tx = Transaction {
        signer_id: "test0".parse().unwrap(),
        receiver_id: "test0".parse().unwrap(),
        public_key: signer.public_key(),
        actions: vec![Action::FunctionCall(Box::new(FunctionCallAction {
            method_name: "write_key_value".to_string(),
            args,
            gas: 10u64.pow(14),
            deposit: 0,
        }))],

        nonce: 0,
        block_hash: CryptoHash::default(),
    };

    // run the transaction, check that execution fails.
    {
        let tip = env.clients[0].chain.head().unwrap();
        let signed_tx =
            Transaction { nonce: tip.height + 1, block_hash: tip.last_block_hash, ..tx }
                .sign(&signer);
        let tx_hash = signed_tx.get_hash();
        assert_eq!(env.clients[0].process_tx(signed_tx, false, false), ProcessTxResponse::ValidTx);
        for i in 0..epoch_length {
            let block = env.clients[0].produce_block(tip.height + i + 1).unwrap().unwrap();
            env.process_block(0, block.clone(), Provenance::PRODUCED);
        }
        let final_result = env.clients[0].chain.get_final_transaction_result(&tx_hash).unwrap();
        assert_matches!(
            final_result.status,
            FinalExecutionStatus::Failure(TxExecutionError::ActionError(_))
        );
    }

    // Run transaction where storage key exactly fits the new limit, check that execution succeeds.
    {
        let args: Vec<u8> = vec![1u8; new_storage_key_limit]
            .into_iter()
            .chain(unc_primitives::test_utils::encode(&[20u64]).into_iter())
            .collect();
        let tx = Transaction {
            signer_id: "test0".parse().unwrap(),
            receiver_id: "test0".parse().unwrap(),
            public_key: signer.public_key(),
            actions: vec![Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: "write_key_value".to_string(),
                args,
                gas: 10u64.pow(14),
                deposit: 0,
            }))],

            nonce: 0,
            block_hash: CryptoHash::default(),
        };
        let tip = env.clients[0].chain.head().unwrap();
        let signed_tx =
            Transaction { nonce: tip.height + 1, block_hash: tip.last_block_hash, ..tx }
                .sign(&signer);
        let tx_hash = signed_tx.get_hash();
        assert_eq!(env.clients[0].process_tx(signed_tx, false, false), ProcessTxResponse::ValidTx);
        for i in 0..epoch_length {
            let block = env.clients[0].produce_block(tip.height + i + 1).unwrap().unwrap();
            env.process_block(0, block.clone(), Provenance::PRODUCED);
        }
        let final_result = env.clients[0].chain.get_final_transaction_result(&tx_hash).unwrap();
        assert_matches!(final_result.status, FinalExecutionStatus::SuccessValue(_));
    }
}
