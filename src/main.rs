use std::{
    env,
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};

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

    // 打印当前目录
    let current_dir = env::current_dir().context("failed to get current directory")?;
    println!("Current directory: {}", current_dir.display());

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
                .parse::<u64>()
                .context(".git/objects file header has invalid size: {size}")?;
            let mut reader = reader.take(size);
            match kind {
                Kind::Blob => {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let n = std::io::copy(&mut reader, &mut stdout)
                        .context("write .git/objects file to stdout")?;
                    anyhow::ensure!(n == size, ".git/objects file was not the expected size (expected: {size}, actual: {n})");
                }
                _ => unimplemented!(),
            }
        }
        Commands::HashObject { write, file } => {
            fn write_blob<W: Write>(file: &Path, writer: W) -> Result<String> {
                let stat =
                    fs::metadata(&file).with_context(|| format!("stat {}", file.display()))?;

                let encoder = ZlibEncoder::new(writer, Compression::default());

                let mut writer = HashWriter {
                    writer: encoder,
                    hasher: Sha1::new(),
                };
                write!(writer, "blob {}\0", stat.len())?;
                let mut f = fs::File::open(file).context("open file")?;
                std::io::copy(&mut f, &mut writer).context("write blob object to writer")?;
                let compressed = writer.writer.finish().context("finish zlib compression")?;
                let hash = writer.hasher.finalize();
                Ok(hex::encode(hash))
            }

            let hash = if write {
                let tmp = "temporary";
                let hash = write_blob(
                    &file,
                    fs::File::create(tmp).context("write blog object to temporary file")?,
                )
                .context("write out blob object")?;
                fs::create_dir_all(format!(".git/objects/{}/", &hash[..2]))
                    .context("create subdir of .git/objects")?;
                fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
                    .context("move blob file into .git/objects")?;
                hash
            } else {
                write_blob(&file, std::io::sink())?
            };
            println!("{}", hash);
        }
    }

    Ok(())
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
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
