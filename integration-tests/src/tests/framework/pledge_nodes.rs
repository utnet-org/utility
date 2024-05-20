use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use actix::{Actor, Addr, System};
use futures::{future, FutureExt};
use rand::Rng;
use unc_primitives::num_rational::Ratio;

use crate::genesis_helpers::genesis_hash;
use crate::test_helpers::heavy_test;
use unc-infra.:config::{GenesisExt, TESTING_INIT_BALANCE, TESTING_INIT_PLEDGE};
use unc-infra.:{load_test_config, start_with_config, UncConfig, UNC_BASE};
use unc_actix_test_utils::run_actix;
use unc_chain_configs::Genesis;
use unc_client::{ClientActor, GetBlock, ProcessTxRequest, Query, Status, ViewClientActor};
use unc_crypto::{InMemorySigner, KeyType};
use unc_network::tcp;
use unc_network::test_utils::{convert_boot_nodes, WaitOrTimeoutActor};
use unc_o11y::testonly::init_integration_logger;
use unc_primitives::hash::CryptoHash;
use unc_primitives::transaction::SignedTransaction;
use unc_primitives::types::{AccountId, BlockHeightDelta, BlockReference, NumSeats};
use unc_primitives::views::{QueryRequest, QueryResponseKind, ValidatorInfo};

use unc_o11y::WithSpanContextExt;
use {primitive_types::U256, unc_primitives::types::BlockId};

#[derive(Clone)]
struct TestNode {
    account_id: AccountId,
    signer: Arc<InMemorySigner>,
    config: UncConfig,
    client: Addr<ClientActor>,
    view_client: Addr<ViewClientActor>,
    genesis_hash: CryptoHash,
}

fn init_test_staking(
    paths: Vec<&Path>,
    num_node_seats: NumSeats,
    num_validator_seats: NumSeats,
    epoch_length: BlockHeightDelta,
    enable_rewards: bool,
    minimum_pledge_divisor: u64,
    state_snapshot_enabled: bool,
    track_all_shards: bool,
) -> Vec<TestNode> {
    init_integration_logger();

    let seeds = (0..num_node_seats).map(|i| format!("unc.{}", i)).collect::<Vec<_>>();
    let mut genesis =
        Genesis::test(seeds.iter().map(|s| s.parse().unwrap()).collect(), num_validator_seats);
    genesis.config.epoch_length = epoch_length;
    genesis.config.num_block_producer_seats = num_node_seats;
    genesis.config.block_producer_kickout_threshold = 20;
    genesis.config.chunk_producer_kickout_threshold = 20;
    genesis.config.minimum_pledge_divisor = minimum_pledge_divisor;
    if !enable_rewards {
        genesis.config.max_inflation_rate = Ratio::from_integer(0);
        genesis.config.min_gas_price = 0;
    }
    let first_node = tcp::ListenerAddr::reserve_for_test();

    let configs = (0..num_node_seats).map(|i| {
        let mut config = load_test_config(
            &format!("unc.{}", i),
            if i == 0 { first_node } else { tcp::ListenerAddr::reserve_for_test() },
            genesis.clone(),
        );
        // Disable state snapshots, because they don't work with epochs that are too short.
        // And they are not needed in these tests.
        config.config.store.state_snapshot_enabled = state_snapshot_enabled;
        if track_all_shards {
            config.config.tracked_shards = vec![0];
        }
        if i != 0 {
            config.network_config.peer_store.boot_nodes =
                convert_boot_nodes(vec![("alice", *first_node)]);
        }
        config.client_config.min_num_peers = num_node_seats as usize - 1;
        config
    });
    configs
        .enumerate()
        .map(|(i, config)| {
            let genesis_hash = genesis_hash(&config.genesis);
            let unc-infra.:UncNode { client, view_client, .. } =
                start_with_config(paths[i], config.clone()).expect("start_with_config");
            let account_id = format!("unc.{}", i).parse::<AccountId>().unwrap();
            let signer = Arc::new(InMemorySigner::from_seed(
                account_id.clone(),
                KeyType::ED25519,
                account_id.as_ref(),
            ));
            TestNode { account_id, signer, config, client, view_client, genesis_hash }
        })
        .collect()
}

/// Runs one validator network, sends staking transaction for the second node and
/// waits until it becomes a validator.
#[test]
#[cfg_attr(not(feature = "expensive_tests"), ignore)]
fn test_pledge_nodes() {
    heavy_test(|| {
        let num_nodes = 2;
        let dirs = (0..num_nodes)
            .map(|i| {
                tempfile::Builder::new().prefix(&format!("pledge_node_{}", i)).tempdir().unwrap()
            })
            .collect::<Vec<_>>();
        run_actix(async {
            let test_nodes = init_test_staking(
                dirs.iter().map(|dir| dir.path()).collect::<Vec<_>>(),
                num_nodes,
                1,
                10,
                false,
                10,
                false,
                false,
            );

            let tx = SignedTransaction::pledge(
                1,
                test_nodes[1].account_id.clone(),
                // &*test_nodes[1].config.block_producer.as_ref().unwrap().signer,
                &*test_nodes[1].signer,
                TESTING_INIT_PLEDGE,
                test_nodes[1].config.validator_signer.as_ref().unwrap().public_key(),
                test_nodes[1].genesis_hash,
            );
            actix::spawn(
                test_nodes[0]
                    .client
                    .send(
                        ProcessTxRequest {
                            transaction: tx,
                            is_forwarded: false,
                            check_only: false,
                        }
                        .with_span_context(),
                    )
                    .map(drop),
            );

            WaitOrTimeoutActor::new(
                Box::new(move |_ctx| {
                    let actor = test_nodes[0].client.send(
                        Status { is_health_check: false, detailed: false }.with_span_context(),
                    );
                    let actor = actor.then(|res| {
                        let res = res.unwrap();
                        if res.is_err() {
                            return future::ready(());
                        }
                        let mut validators = res.unwrap().validators;
                        validators.sort_unstable_by(|a, b| a.account_id.cmp(&b.account_id));
                        if validators
                            == vec![
                                ValidatorInfo {
                                    account_id: "alice".parse().unwrap(),
                                    is_slashed: false,
                                },
                                ValidatorInfo {
                                    account_id: "bob".parse().unwrap(),
                                    is_slashed: false,
                                },
                            ]
                        {
                            System::current().stop();
                        }
                        future::ready(())
                    });
                    actix::spawn(actor);
                }),
                100,
                40000,
            )
            .start();
        });
    });
}

#[test]
#[cfg_attr(not(feature = "expensive_tests"), ignore)]
fn test_validator_kickout() {
    heavy_test(|| {
        let num_nodes = 4;
        let dirs = (0..num_nodes)
            .map(|i| {
                tempfile::Builder::new()
                    .prefix(&format!("validator_kickout_{}", i))
                    .tempdir()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        run_actix(async {
            let test_nodes = init_test_staking(
                dirs.iter().map(|dir| dir.path()).collect::<Vec<_>>(),
                num_nodes,
                4,
                15,
                false,
                (TESTING_INIT_PLEDGE / UNC_BASE) as u64 + 1,
                false,
                false,
            );
            let mut rng = rand::thread_rng();
            let pledges = (0..num_nodes / 2).map(|_| UNC_BASE + rng.gen_range(1..100));
            let pledge_transactions = pledges.enumerate().map(|(i, pledge)| {
                let test_node = &test_nodes[i];
                let signer = Arc::new(InMemorySigner::from_seed(
                    test_node.account_id.clone(),
                    KeyType::ED25519,
                    test_node.account_id.as_ref(),
                ));
                SignedTransaction::pledge(
                    1,
                    test_node.account_id.clone(),
                    &*signer,
                    pledge,
                    test_node.config.validator_signer.as_ref().unwrap().public_key(),
                    test_node.genesis_hash,
                )
            });

            for (i, pledge_transaction) in pledge_transactions.enumerate() {
                let test_node = &test_nodes[i];
                actix::spawn(
                    test_node
                        .client
                        .send(
                            ProcessTxRequest {
                                transaction: pledge_transaction,
                                is_forwarded: false,
                                check_only: false,
                            }
                            .with_span_context(),
                        )
                        .map(drop),
                );
            }

            let finalized_mark: Arc<Vec<_>> =
                Arc::new((0..num_nodes).map(|_| Arc::new(AtomicBool::new(false))).collect());

            WaitOrTimeoutActor::new(
                Box::new(move |_ctx| {
                    let test_nodes = test_nodes.clone();
                    let test_node1 = test_nodes[(num_nodes / 2) as usize].clone();
                    let finalized_mark1 = finalized_mark.clone();

                    let actor = test_node1.client.send(
                        Status { is_health_check: false, detailed: false }.with_span_context(),
                    );
                    let actor = actor.then(move |res| {
                        let expected: Vec<_> = (num_nodes / 2..num_nodes)
                            .map(|i| ValidatorInfo {
                                account_id: AccountId::try_from(format!("unc.{}", i)).unwrap(),
                                is_slashed: false,
                            })
                            .collect();
                        let res = res.unwrap();
                        if res.is_err() {
                            return future::ready(());
                        }
                        if res.unwrap().validators == expected {
                            for i in 0..num_nodes / 2 {
                                let mark = finalized_mark1[i as usize].clone();
                                let actor = test_node1.view_client.send(
                                    Query::new(
                                        BlockReference::latest(),
                                        QueryRequest::ViewAccount {
                                            account_id: test_nodes[i as usize].account_id.clone(),
                                        },
                                    )
                                    .with_span_context(),
                                );
                                let actor =
                                    actor.then(move |res| match res.unwrap().unwrap().kind {
                                        QueryResponseKind::ViewAccount(result) => {
                                            if result.pledging == 0
                                                || result.amount == TESTING_INIT_BALANCE
                                            {
                                                mark.store(true, Ordering::SeqCst);
                                            }
                                            future::ready(())
                                        }
                                        _ => panic!("wrong return result"),
                                    });
                                actix::spawn(actor);
                            }
                            for i in num_nodes / 2..num_nodes {
                                let mark = finalized_mark1[i as usize].clone();

                                let actor = test_node1.view_client.send(
                                    Query::new(
                                        BlockReference::latest(),
                                        QueryRequest::ViewAccount {
                                            account_id: test_nodes[i as usize].account_id.clone(),
                                        },
                                    )
                                    .with_span_context(),
                                );
                                let actor =
                                    actor.then(move |res| match res.unwrap().unwrap().kind {
                                        QueryResponseKind::ViewAccount(result) => {
                                            assert_eq!(result.pledging, TESTING_INIT_PLEDGE);
                                            assert_eq!(
                                                result.amount,
                                                TESTING_INIT_BALANCE - TESTING_INIT_PLEDGE
                                            );
                                            mark.store(true, Ordering::SeqCst);
                                            future::ready(())
                                        }
                                        _ => panic!("wrong return result"),
                                    });
                                actix::spawn(actor);
                            }

                            if finalized_mark1.iter().all(|mark| mark.load(Ordering::SeqCst)) {
                                System::current().stop();
                            }
                        }
                        future::ready(())
                    });
                    actix::spawn(actor);
                }),
                100,
                70000,
            )
            .start();
        });
    })
}

#[test]
#[cfg_attr(not(feature = "expensive_tests"), ignore)]
/// Starts 4 nodes, genesis has 2 validator seats.
/// Node1 unpledges, Node2 pledges.
/// Submit the transactions via Node1 and Node2.
/// Poll `/status` until you see the change of validator assignments.
/// Afterwards check that `pledging` amount on accounts Node1 and Node2 are 0 and TESTING_INIT_PLEDGE.
fn test_validator_join() {
    heavy_test(|| {
        let num_nodes = 4;
        let dirs = (0..num_nodes)
            .map(|i| {
                tempfile::Builder::new().prefix(&format!("validator_join_{}", i)).tempdir().unwrap()
            })
            .collect::<Vec<_>>();
        run_actix(async {
            let test_nodes = init_test_staking(
                dirs.iter().map(|dir| dir.path()).collect::<Vec<_>>(),
                num_nodes,
                2,
                30,
                false,
                10,
                false,
                true,
            );
            let signer = Arc::new(InMemorySigner::from_seed(
                test_nodes[1].account_id.clone(),
                KeyType::ED25519,
                test_nodes[1].account_id.as_ref(),
            ));
            let unpledge_transaction = SignedTransaction::pledge(
                1,
                test_nodes[1].account_id.clone(),
                &*signer,
                0,
                test_nodes[1].config.validator_signer.as_ref().unwrap().public_key(),
                test_nodes[1].genesis_hash,
            );

            let signer = Arc::new(InMemorySigner::from_seed(
                test_nodes[2].account_id.clone(),
                KeyType::ED25519,
                test_nodes[2].account_id.as_ref(),
            ));
            let pledge_transaction = SignedTransaction::pledge(
                1,
                test_nodes[2].account_id.clone(),
                &*signer,
                TESTING_INIT_PLEDGE,
                test_nodes[2].config.validator_signer.as_ref().unwrap().public_key(),
                test_nodes[2].genesis_hash,
            );

            actix::spawn(
                test_nodes[1]
                    .client
                    .send(
                        ProcessTxRequest {
                            transaction: unpledge_transaction,
                            is_forwarded: false,
                            check_only: false,
                        }
                        .with_span_context(),
                    )
                    .map(drop),
            );
            actix::spawn(
                test_nodes[0]
                    .client
                    .send(
                        ProcessTxRequest {
                            transaction: pledge_transaction,
                            is_forwarded: false,
                            check_only: false,
                        }
                        .with_span_context(),
                    )
                    .map(drop),
            );

            let (done1, done2) =
                (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
            let (done1_copy1, done2_copy1) = (done1, done2);
            WaitOrTimeoutActor::new(
                Box::new(move |_ctx| {
                    let test_nodes = test_nodes.clone();
                    let test_node1 = test_nodes[0].clone();
                    let (done1_copy2, done2_copy2) = (done1_copy1.clone(), done2_copy1.clone());
                    let actor = test_node1.client.send(
                        Status { is_health_check: false, detailed: false }.with_span_context(),
                    );
                    let actor = actor.then(move |res| {
                        let expected = vec![
                            ValidatorInfo {
                                account_id: "alice".parse().unwrap(),
                                is_slashed: false,
                            },
                            ValidatorInfo {
                                account_id: "unc.2".parse().unwrap(),
                                is_slashed: false,
                            },
                        ];
                        let res = res.unwrap();
                        if res.is_err() {
                            return future::ready(());
                        }
                        if res.unwrap().validators == expected {
                            let actor = test_node1.view_client.send(
                                Query::new(
                                    BlockReference::latest(),
                                    QueryRequest::ViewAccount {
                                        account_id: test_nodes[1].account_id.clone(),
                                    },
                                )
                                .with_span_context(),
                            );
                            let actor = actor.then(move |res| match res.unwrap().unwrap().kind {
                                QueryResponseKind::ViewAccount(result) => {
                                    if result.pledging == 0 {
                                        done1_copy2.store(true, Ordering::SeqCst);
                                    }
                                    future::ready(())
                                }
                                _ => panic!("wrong return result"),
                            });
                            actix::spawn(actor);
                            let actor = test_node1.view_client.send(
                                Query::new(
                                    BlockReference::latest(),
                                    QueryRequest::ViewAccount {
                                        account_id: test_nodes[2].account_id.clone(),
                                    },
                                )
                                .with_span_context(),
                            );
                            let actor = actor.then(move |res| match res.unwrap().unwrap().kind {
                                QueryResponseKind::ViewAccount(result) => {
                                    if result.pledging == TESTING_INIT_PLEDGE {
                                        done2_copy2.store(true, Ordering::SeqCst);
                                    }

                                    future::ready(())
                                }
                                _ => panic!("wrong return result"),
                            });
                            actix::spawn(actor);
                        }

                        future::ready(())
                    });
                    actix::spawn(actor);
                    if done1_copy1.load(Ordering::SeqCst) && done2_copy1.load(Ordering::SeqCst) {
                        System::current().stop();
                    }
                }),
                1000,
                60000,
            )
            .start();
        });
    });
}

#[test]
#[cfg_attr(not(feature = "expensive_tests"), ignore)]
/// Checks that during the first epoch, total_supply matches total_supply in genesis.
/// Checks that during the second epoch, total_supply matches the expected inflation rate.
fn test_inflation() {
    heavy_test(|| {
        let num_nodes = 1;
        let dirs = (0..num_nodes)
            .map(|i| {
                tempfile::Builder::new().prefix(&format!("pledge_node_{}", i)).tempdir().unwrap()
            })
            .collect::<Vec<_>>();
        let epoch_length = 10;
        run_actix(async {
            let test_nodes = init_test_staking(
                dirs.iter().map(|dir| dir.path()).collect::<Vec<_>>(),
                num_nodes,
                1,
                epoch_length,
                true,
                10,
                false,
                false,
            );
            let initial_total_supply = test_nodes[0].config.genesis.config.total_supply;
            let max_inflation_rate = test_nodes[0].config.genesis.config.max_inflation_rate;

            let (done1, done2) =
                (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
            let (done1_copy1, done2_copy1) = (done1, done2);
            WaitOrTimeoutActor::new(
                Box::new(move |_ctx| {
                    let (done1_copy2, done2_copy2) = (done1_copy1.clone(), done2_copy1.clone());
                    let actor =
                        test_nodes[0].view_client.send(GetBlock::latest().with_span_context());
                    let actor = actor.then(move |res| {
                        if let Ok(Ok(block)) = res {
                            if block.header.height >= 2 && block.header.height <= epoch_length {
                                tracing::info!(?block.header.total_supply, ?block.header.height, ?initial_total_supply, epoch_length, "Step1: epoch1");
                                if block.header.total_supply == initial_total_supply {
                                    done1_copy2.store(true, Ordering::SeqCst);
                                }
                            } else {
                                tracing::info!("Step1: not epoch1");
                            }
                        }
                        future::ready(())
                    });
                    actix::spawn(actor);
                    let view_client = test_nodes[0].view_client.clone();
                    actix::spawn(async move {
                        if let Ok(Ok(block)) =
                            view_client.send(GetBlock::latest().with_span_context()).await
                        {
                            if block.header.height > epoch_length
                                && block.header.height < epoch_length * 2
                            {
                                tracing::info!(?block.header.total_supply, ?block.header.height, ?initial_total_supply, epoch_length, "Step2: epoch2");
                                // It's expected that validator will miss first chunk, hence will only be 95% online, getting 5/9 of their reward.
                                // +10% of protocol reward = 60% of max inflation are allocated.
                                let base_reward = {
                                    let genesis_block_view = view_client
                                        .send(
                                            GetBlock(BlockReference::BlockId(BlockId::Height(0)))
                                                .with_span_context(),
                                        )
                                        .await
                                        .unwrap()
                                        .unwrap();
                                    let epoch_end_block_view = view_client
                                        .send(
                                            GetBlock(BlockReference::BlockId(BlockId::Height(
                                                epoch_length,
                                            )))
                                            .with_span_context(),
                                        )
                                        .await
                                        .unwrap()
                                        .unwrap();
                                    (U256::from(initial_total_supply)
                                        * U256::from(
                                            epoch_end_block_view.header.timestamp_nanosec
                                                - genesis_block_view.header.timestamp_nanosec,
                                        )
                                        * U256::from(*max_inflation_rate.numer() as u64)
                                        / (U256::from(10u64.pow(9) * 365 * 24 * 60 * 60)
                                            * U256::from(*max_inflation_rate.denom() as u64)))
                                    .as_u128()
                                };
                                // To match rounding, split into protocol reward and validator reward.
                                let protocol_reward = base_reward * 1 / 10;
                                let inflation =
                                    base_reward * 1 / 10 + (base_reward - protocol_reward) * 5 / 9;
                                tracing::info!(?block.header.total_supply, ?block.header.height, ?initial_total_supply, epoch_length, ?inflation, "Step2: epoch2");
                                if block.header.total_supply == initial_total_supply + inflation {
                                    done2_copy2.store(true, Ordering::SeqCst);
                                }
                            } else {
                                tracing::info!("Step2: not epoch2");
                            }
                        }
                    });
                    if done1_copy1.load(Ordering::SeqCst) && done2_copy1.load(Ordering::SeqCst) {
                        System::current().stop();
                    }
                }),
                100,
                20000,
            )
            .start();
        });
    });
}
