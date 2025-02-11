use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read, Write},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;

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

#[derive(Debug)]
enum Kind {
    Blob,
    Tree,
    Commit,
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
        } => {
            anyhow::ensure!(pretty_print, "pretty printing is not yet implemented");
            let f = fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[0..2],
                &object_hash[2..]
            ))
            .context("read in .git/objects")?;
            let file_size = f.metadata()?.len() as usize;

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
                anyhow::bail!(
                    ".git/objects file header did not start with a known type: '{header}'"
                );
            };
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => anyhow::bail!("we do not yet know how to print a '{kind}'"),
            };

            let size = size
                .parse::<usize>()
                .context(".git/objects file header has invalid size: {size}")?;
            buf.clear();
            buf.reserve_exact(size);
            buf.resize(size, 0);
            reader
                .read_exact(&mut buf[..])
                .context("read true contents of .git/objects file")?;
            let n = reader
                .read(&mut [0])
                .context("validate EFO in .git/objects file")?;
            anyhow::ensure!(
                n == 0,
                "trailing {n} garbage bytes after .git/objects file contents"
            );
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            match kind {
                Kind::Blob => stdout
                    .write_all(&buf)
                    .context("write object contents to stdout")?,
                _ => unimplemented!(),
            }
        }
    }

    Ok(())
}
