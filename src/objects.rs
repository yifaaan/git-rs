use std::{
    ffi::CStr,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

use anyhow::{Context, Result};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::commands::hash_object::HashWriter;

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
