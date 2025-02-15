use anyhow::{Context, Result};
use std::ffi::CStr;
use std::io::{BufRead, Read, Write};

use crate::objects::{Kind, Object};

pub fn invoke(name_only: bool, tree_hash: &str) -> Result<()> {
    let mut object = Object::read(tree_hash).context("parse out tree object file")?;

    match object.kind {
        Kind::Tree => {
            let mut buf = Vec::new();
            let mut hash_buf = [0; 20];
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            loop {
                buf.clear();
                let n = object
                    .reader
                    .read_until(0, &mut buf)
                    .context("read mode and name from .git/objects file")?;
                if n == 0 {
                    break;
                }
                object
                    .reader
                    .read_exact(&mut hash_buf[..])
                    .context("read tree entry object hash")?;

                let mode_and_name = CStr::from_bytes_with_nul(&buf)
                    .expect("known there is exactly one nul, and it's at the end");
                let mut bits = mode_and_name.to_bytes().splitn(2, |b| *b == b' ');
                let mode = bits.next().expect("mode not found in .git/objects file");
                let name = bits.next().expect("name not found in .git/objects file");
                if name_only {
                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                } else {
                    let hash = hex::encode(hash_buf);
                    let object = Object::read(&hash)
                        .with_context(|| format!("read object for tree entry {}", hash))?;
                    write!(
                        stdout,
                        "{:0>6} {} {hash} ",
                        std::str::from_utf8(mode).context("mode is not valid utf-8")?,
                        object.kind
                    )?;
                    stdout
                        .write_all(&name)
                        .context("write tree entry name to stdout")?;
                }

                writeln!(stdout, "").context("write newline to stdout")?;
            }
        }
        _ => anyhow::bail!("don't know how to ls {}", object.kind),
    }
    Ok(())
}
