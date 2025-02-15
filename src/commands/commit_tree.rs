use anyhow::{Context, Result};

use std::{fmt::Write, io::Cursor};

use crate::objects::{Kind, Object};

pub(crate) fn write_commit(
    message: &str,
    tree_hash: &str,
    parent_tree_hash: Option<&str>,
) -> Result<[u8; 20]> {
    let mut commit = String::new();
    writeln!(commit, "tree {}", tree_hash)?;
    if let Some(parent_tree_hash) = parent_tree_hash {
        writeln!(commit, "parent {}", parent_tree_hash)?;
    }
    let author = "root <root@vmi2447354.contaboserver.net>";
    let committer = "root <root@vmi2447354.contaboserver.net>";
    writeln!(commit, "author {}", author)?;
    writeln!(commit, "committer {}", committer)?;
    writeln!(commit, "{}", message)?;
    Object {
        kind: Kind::Commit,
        expected_size: commit.len() as u64,
        reader: Cursor::new(commit),
    }
    .write_to_objects()
    .context("write commit object")
}

pub fn invoke(message: String, tree_hash: String, parent_tree_hash: Option<String>) -> Result<()> {
    let hash = write_commit(&message, &tree_hash, parent_tree_hash.as_deref())?;
    Ok(())
}
