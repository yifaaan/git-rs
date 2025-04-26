use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ini::Ini;

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
    /// Initialize a new, empty repository.
    Init {
        /// Where to create the repository.
        path: Option<PathBuf>,
    },

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

#[derive(Debug, Default)]
struct GitRepository {
    work_tree: PathBuf,
    git_dir: PathBuf,
    config: ini::Ini,
}

impl GitRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&mut self, path: impl AsRef<Path>, force: bool) -> Result<()> {
        self.work_tree = path.as_ref().to_path_buf();
        // println!("work_tree = {}", work_tree.display());
        self.git_dir = path.as_ref().join(".git");

        if !(force || self.git_dir.is_dir()) {
            bail!("Not a Git repository {}", path.as_ref().display());
        }

        let config_path = repo_file(self, &[&self.git_dir], false)?;
        let mut config;
        if config_path.exists() {
            config = Ini::load_from_str(config_path.to_str().context("Invalid config path")?)?;
        } else if !force {
            bail!("Configuration file missing");
        } else {
            config = Ini::new();
        }

        // TODO: create .git/config
        if !force {
            let core = self
                .config
                .section(Some("core"))
                .context("Failed to get section `core`")?;
            let version = core
                .get("repositoryformatversion")
                .context("Failed to get `repositoryformatversion`")?
                .parse::<u8>()?;
            if version != 0 {
                bail!("Unsupported repositoryformatversion: {version}");
            }
        }
        Ok(())
    }
}

/// Compute path under repo's gitdir.
fn repo_path(git_repo: &GitRepository, paths: &[impl AsRef<Path>]) -> PathBuf {
    let path = paths
        .iter()
        .map(|p| p.as_ref())
        .fold(PathBuf::new(), |acc, p| acc.join(p));
    git_repo.git_dir.join(path)
}

/// Same as repo_path, but create dirname(*path) if absent.
///
/// # Example
/// ```
/// This will create `.git/refs/remotes/origin`.
/// repo_file(r, &["refs", "remotes", "origin", "HEAD"])
/// ```
fn repo_file(git_repo: &GitRepository, paths: &[impl AsRef<Path>], mkdir: bool) -> Result<PathBuf> {
    match repo_dir(git_repo, &paths[0..paths.len() - 1], mkdir) {
        Ok(_) => Ok(repo_path(git_repo, paths)),
        Err(e) => Err(e),
    }
}

/// Same as `repo_path``, but mkdir `paths`` if absent if `mkdir`.
fn repo_dir(git_repo: &GitRepository, paths: &[impl AsRef<Path>], mkdir: bool) -> Result<PathBuf> {
    let path = repo_path(git_repo, paths);
    if path.exists() {
        if path.is_dir() {
            return Ok(path);
        } else {
            bail!("Not a directory {}", path.display());
        }
    }

    if mkdir {
        fs::create_dir_all(&path)?;
        println!("create dir {} in repo_dir", path.display());
        return Ok(path);
    }
    Ok(PathBuf::new())
}

fn repo_create(path: impl AsRef<Path>) -> Result<GitRepository> {
    let mut git_repo = GitRepository::new();
    git_repo.build(path.as_ref(), true)?;

    if git_repo.work_tree.exists() {
        if !git_repo.work_tree.is_dir() {
            bail!("{} is not a directory", path.as_ref().display());
        }
        if git_repo.git_dir.exists() && fs::read_dir(&git_repo.git_dir).iter().count() > 0 {
            bail!("{} is not emptry", git_repo.git_dir.display());
        }
    } else {
        fs::create_dir_all(&git_repo.work_tree)?;
        println!("create dir {}", git_repo.work_tree.display());
    }

    repo_dir(&git_repo, &["branches"], true)?;
    repo_dir(&git_repo, &["objects"], true)?;
    repo_dir(&git_repo, &["refs", "tags"], true)?;
    repo_dir(&git_repo, &["refs", "heads"], true)?;

    let mut f = fs::File::options()
        .write(true)
        .create(true)
        .open(repo_file(&git_repo, &["description"], false)?)?;
    f.write_all(b"Unnamed repository; edit this file 'description' to name the repository.\n")?;

    let mut f = fs::File::options()
        .write(true)
        .create(true)
        .open(repo_file(&git_repo, &["HEAD"], false)?)?;
    f.write_all(b"ref: refs/heads/master\n")?;

    let config_path = repo_file(&git_repo, &["config"], false)?;

    let mut conf = Ini::new();
    conf.with_section(Some("core"))
        .set("repositoryformatversion", "0")
        .set("filemode", "false")
        .set("bare", "false");
    conf.write_to_file(config_path.to_str().context("Invalid config path")?)?;

    Ok(git_repo)
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Commands::Init { path } => {
            if let Some(p) = path {
                repo_create(p)?;
            } else {
                repo_create(".")?;
            }
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
