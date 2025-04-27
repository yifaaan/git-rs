use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use sha1::{Digest, Sha1};

use crate::{
    objects::{object_hash, Object},
    repository::repo_find,
    ObjectType,
};

pub(crate) struct HashWriter<W> {
    pub(crate) writer: W,
    pub(crate) hasher: Sha1,
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub(crate) fn invoke(write: bool, file: &Path) -> Result<()> {
    let object = Object::blob_from_file(file).context("open blob input file")?;
    let hash = if write {
        object
            .write_to_objects()
            .context("write blob object to .git/objects")?
    } else {
        let hash = object
            .write(std::io::sink())
            .context("write out blob object")?;
        hash
    };
    println!("{}", hex::encode(hash));
    Ok(())
}

pub(crate) fn cmd_hash_object(write: bool, object_type: ObjectType, file: PathBuf) -> Result<()> {
    let mut repo = None;
    if write {
        repo = Some(repo_find(".", true)?);
    }

    let data = object_hash(repo, file, object_type)?;
    println!("{:?}", data);
    Ok(())
}
