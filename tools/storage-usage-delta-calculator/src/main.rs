use std::fs::File;
use tracing::debug;
use unc_chain_configs::{Genesis, GenesisValidationMode};
use unc_parameters::RuntimeConfigStore;
use unc_primitives::state_record::StateRecord;
use unc_primitives::version::PROTOCOL_VERSION;
use unc_store::genesis::compute_genesis_storage_usage;

/// Calculates delta between actual storage usage and one saved in state
/// output.json should contain dump of current state,
/// run 'unc-node --home ~/.unc/mainnet/ view_state dump_state'
/// to get it
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = unc_o11y::EnvFilterBuilder::from_env().verbose(Some("")).finish().unwrap();
    let _subscriber = unc_o11y::default_subscriber(env_filter, &Default::default()).global();
    debug!(target: "storage-calculator", "Start");

    let genesis = Genesis::from_file("output.json", GenesisValidationMode::Full)?;
    debug!(target: "storage-calculator", "Genesis read");

    let config_store = RuntimeConfigStore::new(None);
    let config = config_store.get_config(PROTOCOL_VERSION);
    let storage_usage_config = &config.fees.storage_usage_config;
    let storage_usage = compute_genesis_storage_usage(&genesis, storage_usage_config);
    debug!(target: "storage-calculator", "Storage usage calculated");

    let mut result = Vec::new();
    genesis.for_each_record(|record| {
        if let StateRecord::Account { account_id, account } = record {
            let actual_storage_usage = storage_usage.get(account_id).unwrap();
            let saved_storage_usage = account.storage_usage();
            let delta = actual_storage_usage - saved_storage_usage;
            if delta != 0 {
                debug!("{},{}", account_id, delta);
                result.push((account_id.clone(), delta));
            }
        }
    });
    serde_json::to_writer_pretty(&File::create("storage_usage_delta.json")?, &result)?;
    Ok(())
}
