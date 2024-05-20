use std::path::Path;

#[derive(clap::Args)]
pub(crate) struct RunMigrationsCommand {}

impl RunMigrationsCommand {
    pub(crate) fn run(&self, home_dir: &Path) -> anyhow::Result<()> {
        let mut unc_config = unc_infra::config::load_config(
            &home_dir,
            unc_chain_configs::GenesisValidationMode::UnsafeFast,
        )
        .unwrap_or_else(|e| panic!("Error loading config: {:#}", e));
        unc_infra::open_storage(home_dir, &mut unc_config)?;
        Ok(())
    }
}
