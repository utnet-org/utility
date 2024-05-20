mod concurrency;
mod fetch_chain;
mod network;

use anyhow::{anyhow, Context};
use unc_infra::config;
use unc_infra::config::UncConfig;
use network::Network;
use openssl_probe;
use std::sync::Arc;
use unc_async::actix::AddrWithAutoSpanContextExt;
use unc_async::messaging::LateBoundSender;
use unc_async::messaging::Sender;
use unc_async::time;
use unc_chain_configs::Genesis;
use unc_network::concurrency::ctx;
use unc_network::concurrency::scope;
use unc_network::PeerManagerActor;
use unc_o11y::tracing::{error, info};
use unc_primitives::block::GenesisId;
use unc_primitives::hash::CryptoHash;

fn genesis_hash(chain_id: &str) -> CryptoHash {
    return match chain_id {
        unc_primitives::chains::MAINNET => "EPnLgE7iEq9s7yTkos96M3cWymH5avBAPm3qx3NXqR8H",
        unc_primitives::chains::TESTNET => "FWJ9kR6KFWoyMoNjpLXXGHeuiy7tEY6GmoFeCA5yuc6b",
        _ => {
            return Default::default();
        }
    }
    .parse()
    .unwrap();
}

pub fn start_with_config(config: UncConfig, qps_limit: u32) -> anyhow::Result<Arc<Network>> {
    let network_adapter = Arc::new(LateBoundSender::default());
    let network = Network::new(&config, network_adapter.clone().into(), qps_limit);

    let network_actor = PeerManagerActor::spawn(
        time::Clock::real(),
        unc_store::db::TestDB::new(),
        config.network_config,
        network.clone(),
        Sender::noop(),
        GenesisId {
            chain_id: config.client_config.chain_id.clone(),
            hash: genesis_hash(&config.client_config.chain_id),
        },
    )
    .context("PeerManagerActor::spawn()")?;
    network_adapter.bind(network_actor.with_auto_span_context());
    return Ok(network);
}

fn download_configs(chain_id: &str, dir: &std::path::Path) -> anyhow::Result<UncConfig> {
    // Always fetch the config.
    std::fs::create_dir_all(dir)?;
    let url = config::get_config_url(chain_id);
    let config_path = &dir.join(config::CONFIG_FILENAME);
    config::download_config(&url, config_path)?;
    let config = config::Config::from_file(config_path)?;

    // Generate node key.
    let account_id = "node".parse().unwrap();
    let node_signer =
        unc_crypto::InMemorySigner::from_random(account_id, unc_crypto::KeyType::ED25519);
    let mut genesis = Genesis::default();
    genesis.config.chain_id = chain_id.to_string();
    UncConfig::new(config, genesis, (&node_signer).into(), None)
}

#[derive(clap::Parser, Debug)]
struct Cmd {
    #[clap(long)]
    pub chain_id: String,
    #[clap(long)]
    pub start_block_hash: String,
    #[clap(long, default_value = "200")]
    pub qps_limit: u32,
    #[clap(long, default_value = "2000")]
    pub block_limit: u64,
}

impl Cmd {
    fn parse_and_run() -> anyhow::Result<()> {
        let cmd: Self = clap::Parser::parse();
        let start_block_hash =
            cmd.start_block_hash.parse::<CryptoHash>().map_err(|x| anyhow!(x.to_string()))?;

        let mut cache_dir = dirs::cache_dir().context("dirs::cache_dir() = None")?;
        cache_dir.push("unc_configs");
        cache_dir.push(&cmd.chain_id);

        info!("downloading configs for chain {}", cmd.chain_id);
        let home_dir = cache_dir.as_path();
        let unc_config =
            download_configs(&cmd.chain_id, home_dir).context("Failed to initialize configs")?;

        info!("#boot nodes = {}", unc_config.network_config.peer_store.boot_nodes.len());
        // Dropping Runtime is blocking, while futures should never be blocking.
        // Tokio has a runtime check which panics if you drop tokio Runtime from a future executed
        // on another Tokio runtime.
        // To avoid that, we create a runtime within the synchronous code and pass just an Arc
        // inside of it.
        let rt_ = Arc::new(tokio::runtime::Runtime::new()?);
        let rt = rt_;
        return actix::System::new().block_on(async move {
            let network =
                start_with_config(unc_config, cmd.qps_limit).context("start_with_config")?;

            // We execute the chain_sync on a totally separate set of system threads to minimize
            // the interaction with actix.
            rt.spawn(async move {
                scope::run!(|s| async {
                    s.spawn_bg(async {
                        match ctx::wait(tokio::signal::ctrl_c()).await {
                            Err(ctx::ErrCanceled) => Ok(()),
                            Ok(res) => {
                                res?;
                                info!("Got CTRL+C, stopping...");
                                Err(anyhow!("Got CTRL+C"))
                            }
                        }
                    });
                    fetch_chain::run(&network, start_block_hash, cmd.block_limit).await?;
                    info!("Fetch completed");
                    anyhow::Ok(())
                })
            })
            .await??;
            return Ok(());
        });
    }
}

fn main() {
    let env_filter = unc_o11y::EnvFilterBuilder::from_env()
        .finish()
        .unwrap()
        .add_directive(unc_o11y::tracing::Level::INFO.into());
    let _subscriber = unc_o11y::default_subscriber(env_filter, &Default::default()).global();
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));
    openssl_probe::init_ssl_cert_env_vars();
    if let Err(e) = Cmd::parse_and_run() {
        error!("Cmd::parse_and_run(): {:#}", e);
    }
}
