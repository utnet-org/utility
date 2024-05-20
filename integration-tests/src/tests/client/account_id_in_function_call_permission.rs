use unc_infra::config::GenesisExt;
use unc_infra::test_utils::TestEnvNightshadeSetupExt;
use unc_chain::ChainGenesis;
use unc_chain_configs::Genesis;
use unc_client::test_utils::TestEnv;
use unc_client::ProcessTxResponse;
use unc_crypto::{InMemorySigner, KeyType, Signer};
use unc_parameters::RuntimeConfigStore;
use unc_primitives::account::{AccessKey, AccessKeyPermission, FunctionCallPermission};
use unc_primitives::errors::{ActionsValidationError, InvalidTxError};
use unc_primitives::hash::CryptoHash;
use unc_primitives::transaction::{Action, AddKeyAction, Transaction};

#[test]
fn test_account_id_in_function_call_permission_upgrade() {
    // The immediate protocol upgrade needs to be set for this test to pass in
    // the release branch where the protocol upgrade date is set.
    std::env::set_var("UNC_TESTS_IMMEDIATE_PROTOCOL_UPGRADE", "1");

    let old_protocol_version =
        unc_primitives::version::ProtocolFeature::AccountIdInFunctionCallPermission
            .protocol_version()
            - 1;
    let new_protocol_version = old_protocol_version + 1;

    // Prepare TestEnv with a contract at the old protocol version.
    let mut env = {
        let epoch_length = 5;
        let mut genesis =
            Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
        genesis.config.epoch_length = epoch_length;
        genesis.config.protocol_version = old_protocol_version;
        let chain_genesis = ChainGenesis::new(&genesis);
        TestEnv::builder(chain_genesis)
            .real_epoch_managers(&genesis.config)
            .nightshade_runtimes_with_runtime_config_store(
                &genesis,
                vec![RuntimeConfigStore::new(None)],
            )
            .build()
    };

    let signer = InMemorySigner::from_seed("test0".parse().unwrap(), KeyType::ED25519, "test0");
    let tx = Transaction {
        signer_id: "test0".parse().unwrap(),
        receiver_id: "test0".parse().unwrap(),
        public_key: signer.public_key(),
        actions: vec![Action::AddKey(Box::new(AddKeyAction {
            public_key: signer.public_key(),
            access_key: AccessKey {
                nonce: 1,
                permission: AccessKeyPermission::FunctionCall(FunctionCallPermission {
                    allowance: None,
                    receiver_id: "#".to_string(),
                    method_names: vec![],
                }),
            },
        }))],
        nonce: 0,
        block_hash: CryptoHash::default(),
    };

    // Run the transaction, it should pass as we don't do validation at this protocol version.
    {
        let tip = env.clients[0].chain.head().unwrap();
        let signed_transaction =
            Transaction { nonce: 10, block_hash: tip.last_block_hash, ..tx.clone() }.sign(&signer);
        assert_eq!(
            env.clients[0].process_tx(signed_transaction, false, false),
            ProcessTxResponse::ValidTx
        );
        for i in 0..3 {
            env.produce_block(0, tip.height + i + 1);
        }
    };

    env.upgrade_protocol(new_protocol_version);

    // Re-run the transaction, now it fails due to invalid account id.
    {
        let tip = env.clients[0].chain.head().unwrap();
        let signed_transaction =
            Transaction { nonce: 11, block_hash: tip.last_block_hash, ..tx }.sign(&signer);
        assert_eq!(
            env.clients[0].process_tx(signed_transaction, false, false),
            ProcessTxResponse::InvalidTx(InvalidTxError::ActionsValidation(
                ActionsValidationError::InvalidAccountId { account_id: "#".to_string() }
            ))
        )
    };
}

#[test]
fn test_very_long_account_id() {
    let mut env = {
        let genesis = Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
        let chain_genesis = ChainGenesis::new(&genesis);
        TestEnv::builder(chain_genesis)
            .real_epoch_managers(&genesis.config)
            .nightshade_runtimes_with_runtime_config_store(
                &genesis,
                vec![RuntimeConfigStore::new(None)],
            )
            .build()
    };

    let tip = env.clients[0].chain.head().unwrap();
    let signer = InMemorySigner::from_seed("test0".parse().unwrap(), KeyType::ED25519, "test0");
    let tx = Transaction {
        signer_id: "test0".parse().unwrap(),
        receiver_id: "test0".parse().unwrap(),
        public_key: signer.public_key(),
        actions: vec![Action::AddKey(Box::new(AddKeyAction {
            public_key: signer.public_key(),
            access_key: AccessKey {
                nonce: 1,
                permission: AccessKeyPermission::FunctionCall(FunctionCallPermission {
                    allowance: None,
                    receiver_id: "A".repeat(1024),
                    method_names: vec![],
                }),
            },
        }))],
        nonce: 0,
        block_hash: tip.last_block_hash,
    }
    .sign(&signer);

    assert_eq!(
        env.clients[0].process_tx(tx, false, false),
        ProcessTxResponse::InvalidTx(InvalidTxError::ActionsValidation(
            ActionsValidationError::InvalidAccountId { account_id: "A".repeat(128) }
        ))
    )
}
