use crate::commands::status::get_status;
use crate::objects::branch_object::Branch;
use crate::objects::commit_object::Commit;
use crate::objects::tree_object::read_tree;
use crate::utils::{HEAD_DIR, OBJ_DIR};
use anyhow::{Context, Result};
use colored::*;
use flate2::bufread::ZlibDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use sha1::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn checkout_command(target: &str, force: bool) -> Result<()> {
    if !force {
        let (added, modified, deleted, untracked) = get_status(Path::new("."))?;
        if !force {
            let (added, modified, deleted, untracked) = get_status(Path::new("."))?;
            if !modified.is_empty() || !deleted.is_empty() || !untracked.is_empty() {
                return Err(anyhow::anyhow!(
                    "You have uncommitted changes. Commit or stash them first (or use --force)"
                        .red()
                        .to_string()
                ));
            }
        }
    }

    let commit_hash = if target.len() == 40 {
        target.to_string()
    } else {
        match Branch::list()?.iter().find(|b| b.name == target) {
            Some(branch) => branch.commit_hash.clone(),
            None => return Err(anyhow::anyhow!("Branch or commit '{}' not found", target)),
        }
    };

    let commit = Commit::load(&commit_hash, &PathBuf::from(&*OBJ_DIR))?;

    clean_working_directory(".")?;

    restore_tree(&commit.tree, Path::new("."))?;

    if target.len() == 40 {
        fs::write(&*HEAD_DIR, commit_hash)?;
    } else {
        fs::write(&*HEAD_DIR, format!("ref: refs/heads/{}\n", target))?;
    }

    println!("Succesfully checked out {}", target);
    Ok(())
}

fn clean_working_directory(path: &str) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("."))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            if path.starts_with(".vcs") || path.starts_with(".git") || path.starts_with("target") {
                continue;
            }
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn restore_tree(tree_hash: &str, base_path: &Path) -> Result<()> {
    let tree = read_tree(tree_hash)?;
    let pb = ProgressBar::new(tree.entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {percent}% • {pos}/{len} files {msg}")
            .unwrap()
            .progress_chars("▰▰▱"),
    );

    for entry in tree.entries {
        let path = base_path.join(&entry.name);
        pb.set_prefix(format!("Processing: {}", entry.name));

        match entry.object_type.as_str() {
            "tree" => {
                fs::create_dir_all(&path)?;
                let _ = restore_tree(&entry.object_hash, &path);
            }
            "blob" => {
                restore_blob(&entry.object_hash, &path)?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown object type: {}",
                    entry.object_type
                ))
            }
        }
        pb.inc(1);
    }
    pb.finish_with_message("Files restored Succesfully!");
    Ok(())
}

fn restore_blob(hash: &str, path: &Path) -> Result<()> {
    if !should_update_file(path, hash) {
        return Ok(());
    }

    let object_path = PathBuf::from(&*OBJ_DIR).join(&hash[..2]).join(&hash[2..]);

    let compressed_data =
        fs::read(&object_path).with_context(|| format!("Failed to read object {}", hash))?;

    let mut decoder = ZlibDecoder::new(&compressed_data[..]);
    let mut decompressed_data = Vec::new();

    decoder.read_to_end(&mut decompressed_data)?;

    let content_start = decompressed_data
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid blob format"))?;

    let content = &decompressed_data[content_start + 1..];

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    Ok(())
}

fn should_update_file(path: &Path, expected_hash: &str) -> bool {
    if !path.exists() {
        return true;
    }

    if let Ok(current_content) = fs::read(path) {
        let mut hasher = Sha1::new();
        hasher.update(&current_content);
        let current_hash = format!("{:x}", hasher.finalize());
        current_hash != expected_hash
    } else {
        true
    }
}
