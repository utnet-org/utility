//! Client is responsible for tracking the chain, chunks, and producing them when needed.
//! This client works completely synchronously and must be operated by some async actor outside.

use std::collections::HashMap;
use std::sync::Arc;
use unc_chain::ChainGenesis;
use unc_chain_configs::Genesis;
use unc_client::test_utils::TestEnv;
use unc_crypto::KeyType;
use unc_infra::config::GenesisExt;
use unc_infra::test_utils::TestEnvNightshadeSetupExt;
use unc_network::test_utils::MockPeerManagerAdapter;
use unc_primitives::block::{Approval, ApprovalInner};
use unc_primitives::block_header::ApprovalType;
use unc_primitives::hash::hash;
use unc_primitives::network::PeerId;
use unc_primitives::test_utils::create_test_signer;
use unc_primitives::validator_signer::InMemoryValidatorSigner;

#[test]
fn test_pending_approvals() {
    let genesis = Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
    let mut env = TestEnv::builder(ChainGenesis::test())
        .real_epoch_managers(&genesis.config)
        .nightshade_runtimes(&genesis)
        .build();
    let signer = create_test_signer("test0");
    let parent_hash = hash(&[1]);
    let approval = Approval::new(parent_hash, 0, 1, &signer);
    let peer_id = PeerId::random();
    env.clients[0].collect_block_approval(&approval, ApprovalType::PeerApproval(peer_id.clone()));
    let approvals = env.clients[0].pending_approvals.pop(&ApprovalInner::Endorsement(parent_hash));
    let expected =
        vec![("test0".parse().unwrap(), (approval, ApprovalType::PeerApproval(peer_id)))]
            .into_iter()
            .collect::<HashMap<_, _>>();
    assert_eq!(approvals, Some(expected));
}

#[test]
fn test_invalid_approvals() {
    let genesis = Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
    let network_adapter = Arc::new(MockPeerManagerAdapter::default());
    let mut env = TestEnv::builder(ChainGenesis::test())
        .real_epoch_managers(&genesis.config)
        .nightshade_runtimes(&genesis)
        .network_adapters(vec![network_adapter])
        .build();
    let signer = create_test_signer("random");
    let parent_hash = hash(&[1]);
    // Approval not from a validator. Should be dropped
    let approval = Approval::new(parent_hash, 1, 3, &signer);
    let peer_id = PeerId::random();
    env.clients[0].collect_block_approval(&approval, ApprovalType::PeerApproval(peer_id.clone()));
    assert_eq!(env.clients[0].pending_approvals.len(), 0);
    // Approval with invalid signature. Should be dropped
    let signer =
        InMemoryValidatorSigner::from_seed("test0".parse().unwrap(), KeyType::ED25519, "random");
    let genesis_hash = *env.clients[0].chain.genesis().hash();
    let approval = Approval::new(genesis_hash, 0, 1, &signer);
    env.clients[0].collect_block_approval(&approval, ApprovalType::PeerApproval(peer_id));
    assert_eq!(env.clients[0].pending_approvals.len(), 0);
}

#[test]
fn test_cap_max_gas_price() {
    use unc_chain::Provenance;
    use unc_primitives::version::ProtocolFeature;
    let mut genesis = Genesis::test(vec!["test0".parse().unwrap(), "test1".parse().unwrap()], 1);
    let epoch_length = 5;
    genesis.config.min_gas_price = 1_000;
    genesis.config.max_gas_price = 1_000_000;
    genesis.config.protocol_version = ProtocolFeature::CapMaxGasPrice.protocol_version();
    genesis.config.epoch_length = epoch_length;
    let chain_genesis = ChainGenesis::new(&genesis);
    let mut env = TestEnv::builder(chain_genesis)
        .real_epoch_managers(&genesis.config)
        .nightshade_runtimes(&genesis)
        .build();

    for i in 1..epoch_length {
        let block = env.clients[0].produce_block(i).unwrap().unwrap();
        env.process_block(0, block, Provenance::PRODUCED);
    }

    let last_block = env.clients[0].chain.get_block_by_height(epoch_length - 1).unwrap();
    let protocol_version = env.clients[0]
        .epoch_manager
        .get_epoch_protocol_version(last_block.header().epoch_id())
        .unwrap();
    let min_gas_price = env.clients[0].chain.block_economics_config.min_gas_price(protocol_version);
    let max_gas_price = env.clients[0].chain.block_economics_config.max_gas_price(protocol_version);
    assert!(max_gas_price <= 20 * min_gas_price);
}
