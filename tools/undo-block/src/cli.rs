use unc_chain::ChainStore;
use unc_chain_configs::GenesisValidationMode;
use unc_epoch_manager::EpochManager;
use unc_store::{Mode, NodeStorage};
use framework::load_config;
use std::path::Path;

#[derive(clap::Parser)]
pub struct UndoBlockCommand {
    /// Only reset the block head to the tail block. Does not reset the header head.
    #[arg(short, long)]
    reset_only_body: bool,
}

impl UndoBlockCommand {
    pub fn run(
        self,
        home_dir: &Path,
        genesis_validation: GenesisValidationMode,
    ) -> anyhow::Result<()> {
        let unc_config = load_config(home_dir, genesis_validation)
            .unwrap_or_else(|e| panic!("Error loading config: {:#}", e));

        let store_opener = NodeStorage::opener(
            home_dir,
            unc_config.config.archive,
            &unc_config.config.store,
            None,
        );

        let storage = store_opener.open_in_mode(Mode::ReadWrite).unwrap();
        let store = storage.get_hot_store();

        let epoch_manager =
            EpochManager::new_arc_handle(store.clone(), &unc_config.genesis.config);

        let mut chain_store = ChainStore::new(
            store,
            unc_config.genesis.config.genesis_height,
            unc_config.client_config.save_trie_changes,
        );

        if self.reset_only_body {
            crate::undo_only_block_head(&mut chain_store, &*epoch_manager)
        } else {
            crate::undo_block(&mut chain_store, &*epoch_manager)
        }
    }
}
