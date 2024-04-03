use std::path::Path;

#[derive(clap::Args)]
pub(crate) struct RunMigrationsCommand {}

impl RunMigrationsCommand {
    pub(crate) fn run(&self, home_dir: &Path) -> anyhow::Result<()> {
        let mut unc_config = framework::config::load_config(
            &home_dir,
            unc_chain_configs::GenesisValidationMode::UnsafeFast,
        )
        .unwrap_or_else(|e| panic!("Error loading config: {:#}", e));
        framework::open_storage(home_dir, &mut unc_config)?;
        Ok(())
    }
}
