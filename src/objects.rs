use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::ZlibEncoder,
    Compression,
};
use sha1::{Digest, Sha1};

use crate::{
    commands::hash_object::HashWriter,
    repository::{repo_file, GitRepository},
};

#[derive(Debug)]
pub(crate) enum Kind {
    Blob,
    Tree,
    Commit,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Tree => write!(f, "tree"),
            Kind::Commit => write!(f, "commit"),
        }
    }
}

pub(crate) struct Object<R> {
    pub(crate) kind: Kind,
    pub(crate) expected_size: u64,
    pub(crate) reader: R,
}

impl Object<()> {
    pub fn blob_from_file(file: impl AsRef<Path>) -> Result<Object<impl Read>> {
        let file = file.as_ref();
        let stat = std::fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
        let file = std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;

        Ok(Object {
            kind: Kind::Blob,
            expected_size: stat.len(),
            reader: file,
        })
    }

    pub fn read(object_hash: &str) -> Result<Object<impl BufRead>> {
        let f = std::fs::File::open(format!(
            ".git/objects/{}/{}",
            &object_hash[0..2],
            &object_hash[2..]
        ))
        .context("read in .git/objects")?;
        let decoder = ZlibDecoder::new(f);
        let mut reader = BufReader::new(decoder);
        let mut buf = Vec::new();
        reader
            .read_until(0, &mut buf)
            .context("read header from .git/objects")?;
        let header = CStr::from_bytes_with_nul(&buf)
            .expect("known there is exactly one nul, and it's at the end");
        let header = header
            .to_str()
            .context(".git/objects file header isn't valid utf-8")?;
        let Some((kind, size)) = header.split_once(' ') else {
            anyhow::bail!(".git/objects file header did not start with a known type: '{header}'");
        };
        let kind = match kind {
            "blob" => Kind::Blob,
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            _ => anyhow::bail!("we do not yet know how to print a '{kind}'"),
        };

        let size = size
            .parse::<u64>()
            .context(".git/objects file header has invalid size: {size}")?;
        let reader = reader.take(size);
        Ok(Object {
            kind,
            expected_size: size,
            reader,
        })
    }
}

impl<R: Read> Object<R> {
    pub(crate) fn write(mut self, writer: impl Write) -> Result<[u8; 20]> {
        let writer = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer,
            hasher: Sha1::new(),
        };
        write!(writer, "{} {}\0", self.kind, self.expected_size)?;
        std::io::copy(&mut self.reader, &mut writer)?;
        let _ = writer.writer.finish()?;
        let hash = writer.hasher.finalize();
        Ok(hash.into())
    }

    /// write the tree object to the objects directory
    pub(crate) fn write_to_objects(self) -> Result<[u8; 20]> {
        let tmp = "temporary";
        let hash = self
            .write(std::fs::File::create(tmp).context("write blog object for tree")?)
            .context("stream file into tree object file")?;
        let hash_hex = hex::encode(hash);
        std::fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
            .context("create subdir of .git/objects")?;
        std::fs::rename(
            tmp,
            format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
        )
        .context("move blob file into .git/objects")?;
        Ok(hash)
    }
}

trait GitObject {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(&self, buf: &[u8]) -> Box<dyn GitObject>;
}

struct GitCommit {}

impl GitCommit {
    fn build<R: std::io::Read>(reader: R) -> Result<Self> {
        todo!()
    }
}

impl GitObject for GitCommit {
    fn deserialize(&self, buf: &[u8]) -> Box<dyn GitObject> {
        todo!()
    }

    fn serialize(&self) -> Vec<u8> {
        todo!()
    }
}

struct GitTree {}

impl GitTree {
    fn build<R: std::io::Read>(reader: R) -> Result<Self> {
        todo!()
    }
}

impl GitObject for GitTree {
    fn deserialize(&self, buf: &[u8]) -> Box<dyn GitObject> {
        todo!()
    }

    fn serialize(&self) -> Vec<u8> {
        todo!()
    }
}
struct GitTag {}

impl GitTag {
    fn build<R: std::io::Read>(reader: R) -> Result<Self> {
        todo!()
    }
}

impl GitObject for GitTag {
    fn deserialize(&self, buf: &[u8]) -> Box<dyn GitObject> {
        todo!()
    }

    fn serialize(&self) -> Vec<u8> {
        todo!()
    }
}

struct GitBlob {}

impl GitBlob {
    fn build<R: std::io::Read>(reader: R) -> Result<Self> {
        todo!()
    }
}

impl GitObject for GitBlob {
    fn deserialize(&self, buf: &[u8]) -> Box<dyn GitObject> {
        todo!()
    }

    fn serialize(&self) -> Vec<u8> {
        todo!()
    }
}

pub fn object_read(git_repo: &GitRepository, sha: &str) -> Result<Option<Box<dyn GitObject>>> {
    let path = repo_file(git_repo, &[&sha[0..2], &sha[2..]], false)?;
    if !path.is_file() {
        return Ok(None);
    }
    use std::io::prelude::*;
    let f = fs::File::open(path)?;
    let mut d = GzDecoder::new(f);
    // let mut decompressed = Vec::new();
    // d.read_to_end(&mut decompressed)?;
    let mut reader = BufReader::new(d);

    let mut obj_type = Vec::new();
    let obj_type_len = reader.read_until(b' ', &mut obj_type)?;
    obj_type.pop();
    let obj_type = std::str::from_utf8(&obj_type)?;

    let mut obj_size = Vec::new();
    let mut obj_size_len = reader.read_until(b'0', &mut obj_size)?;
    obj_size.pop();
    let obj_size = std::str::from_utf8(&obj_size)?.parse::<usize>()?;

    let mut data = Vec::new();
    let data_len = reader.read_to_end(&mut data)?;
    if obj_size != data_len {
        bail!("Malformed object {}: bad length", sha);
    }

    match obj_type {
        "commit" => Ok(Some(Box::new(GitCommit::build(reader)?))),
        "tree" => Ok(Some(Box::new(GitTree::build(reader)?))),
        "tag" => Ok(Some(Box::new(GitTag::build(reader)?))),
        "blob" => Ok(Some(Box::new(GitBlob::build(reader)?))),
        _ => bail!("Unknown object type {}", obj_type),
    }
}
