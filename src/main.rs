use std::{env, fs, path::PathBuf};

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
        } => commands::cat_file::invoke(pretty_print, &object_hash)?,
        Commands::HashObject { write, file } => commands::hash_object::invoke(write, &file)?,
        Commands::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, &tree_hash)?,
    }
    Ok(())
}
