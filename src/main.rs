use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod commands;
mod objects;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Initialize a new project
    Init,

    CatFile {
        /// Pretty print the object
        #[arg(short, long)]
        pretty_print: bool,

        /// Object hash to print
        #[arg(value_parser = validate_object_hash)]
        object_hash: String,
    },

    HashObject {
        /// Write the object into the git database
        #[arg(short, long)]
        write: bool,

        /// File to hash
        file: PathBuf,
    },

    LsTree {
        #[arg(short)]
        name_only: bool,

        /// tree hash to print
        #[arg(value_parser = validate_object_hash)]
        tree_hash: String,
    },

    WriteTree,

    CommitTree {
        #[arg(short)]
        message: String,

        #[arg(short, value_parser = validate_object_hash)]
        parent_tree_hash: Option<String>,

        /// tree hash to print
        #[arg(value_parser = validate_object_hash)]
        tree_hash: String,
    },

    Commit {
        #[arg(short)]
        message: String,
    },
}

/// Validate that the object hash is a valid SHA-1 hash
/// TODO: support shortest-unique object hash
fn validate_object_hash(s: &str) -> Result<String, String> {
    if s.len() != 40 {
        return Err("Object hash must be 40 characters long".to_string());
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Object hash must contain only hexadecimal characters".to_string());
    }
    Ok(s.to_string())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Commands::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git repository in current directory.");
        }
        Commands::CatFile {
            pretty_print,
            object_hash,
        } => commands::cat_file::invoke(pretty_print, object_hash)?,
        Commands::HashObject { write, file } => commands::hash_object::invoke(write, &file)?,
        Commands::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, tree_hash)?,
        Commands::WriteTree => commands::write_tree::invoke()?,
        Commands::CommitTree {
            message,
            parent_tree_hash,
            tree_hash,
        } => commands::commit_tree::invoke(message, tree_hash, parent_tree_hash)?,
        Commands::Commit { message } => {
            let head_ref = std::fs::read_to_string(".git/HEAD").context("read HEAD")?;
            let Some(head_ref) = head_ref.strip_prefix("ref: ") else {
                anyhow::bail!("refusing to commit onto detached HEAD");
            };
            let head_ref = head_ref.trim();

            let parent_hash = std::fs::read_to_string(format!(".git/{head_ref}"))
                .with_context(|| format!("read HEAD reference target {head_ref}"))?;
            let parent_hash = parent_hash.trim();

            let Some(tree_hash) =
                commands::write_tree::write_tree_for(Path::new(".")).context("write tree")?
            else {
                eprintln!("not committing empty tree");
                return Ok(());
            };

            let commit_hash = commands::commit_tree::write_commit(
                &message,
                &hex::encode(tree_hash),
                Some(&hex::encode(parent_hash)),
            )
            .context("create commit")?;
            let commit_hash = hex::encode(commit_hash);
            std::fs::write(format!(".git/{head_ref}"), &commit_hash)
                .with_context(|| format!("update HEAD reference target {head_ref}"))?;
            println!("HEAD is now at {commit_hash}");
        }
    }
    Ok(())
}
