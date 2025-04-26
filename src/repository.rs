use anyhow::{bail, Context, Result};
use ini::Ini;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub struct GitRepository {
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
pub fn repo_path(git_repo: &GitRepository, paths: &[impl AsRef<Path>]) -> PathBuf {
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
pub fn repo_file(
    git_repo: &GitRepository,
    paths: &[impl AsRef<Path>],
    mkdir: bool,
) -> Result<PathBuf> {
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

pub fn repo_create(path: impl AsRef<Path>) -> Result<GitRepository> {
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

/// Find the root of current repository.
fn repo_find(path: impl AsRef<Path>, required: bool) -> Result<GitRepository> {
    fn get_real_path(path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = if path.as_ref().is_symlink() {
            path.as_ref().read_link()?
        } else {
            path.as_ref().to_path_buf()
        };
        Ok(path)
    }
    let path = get_real_path(path)?;

    if path.join(".git").is_dir() {
        let mut repo = GitRepository::new();
        repo.build(path, false)?;
        return Ok(repo);
    }

    let parent = get_real_path(path.join(".."))?;
    if parent == path {
        if required {
            bail!("No git directory");
        }
        return Ok(Default::default());
    }

    return repo_find(parent, required);
}
