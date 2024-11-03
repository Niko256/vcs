use crate::cli::Commands;
use crate::commands::{
    cat_file::cat_file_command,
    hash_object::{hash_object_command, HashObjectArgs},
    index::{ls_files::ls_files_command, rm_index::rm_command},
    init::init_command,
    status,
};
use anyhow::Result;
use std::path::Path;

pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init => {
            init_command()?;
        }
        Commands::CatFile {
            pretty_print,
            object_hash,
            show_type,
            show_size,
        } => {
            cat_file_command(pretty_print, object_hash, show_type, show_size)?;
        }
        Commands::HashObject { file_path } => {
            hash_object_command(HashObjectArgs { file_path })?;
        }
        Commands::Status => {
            status::status_command()?;
        }
        Commands::LsFiles { stage } => {
            ls_files_command(stage)?;
        }
        Commands::Rm { cashed, path } => {
            rm_command(Path::new(&path), cashed)?;
        }
    }
    Ok(())
}
