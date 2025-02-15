use anyhow::{Context, Result};

use std::{io::Cursor, os::unix::fs::PermissionsExt, path::Path};

use crate::objects::{Kind, Object};

fn write_tree_for(path: &Path) -> Result<Option<[u8; 20]>> {
    let mut entries = std::fs::read_dir(path)
        .with_context(|| format!("open directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("bad directory entry in {}", path.display()))?;
    entries.sort_unstable_by(|a, b| {
        let (an, bn) = (a.file_name(), b.file_name());
        let an_bytes = an.as_encoded_bytes();
        let bn_bytes = bn.as_encoded_bytes();

        let min_len = an_bytes.len().min(bn_bytes.len());
        match an_bytes[..min_len].cmp(&bn_bytes[..min_len]) {
            std::cmp::Ordering::Equal => {
                if an_bytes.len() == bn_bytes.len() {
                    std::cmp::Ordering::Equal
                } else {
                    // 如果一个是目录，一个是文件，则认为它们相等
                    let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

                    if (a_is_dir && an_bytes.len() < bn_bytes.len())
                        || (b_is_dir && bn_bytes.len() < an_bytes.len())
                    {
                        std::cmp::Ordering::Equal
                    } else if an_bytes.len() < bn_bytes.len() {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                }
            }
            other => other,
        }
    });

    // get all the files and directories in the directory for the tree object
    let mut tree_object = Vec::new();
    for entry in entries {
        let file_name = entry.file_name();
        if file_name == ".git" {
            continue;
        }
        let meta = entry.metadata().context("get metadata")?;
        let mode = if meta.is_dir() {
            "40000"
        } else if meta.is_symlink() {
            "120000"
        } else if meta.permissions().mode() & 0o111 != 0 {
            "100755"
        } else {
            "100644"
        };
        let hash = if meta.is_dir() {
            if let Some(hash) = write_tree_for(&entry.path())? {
                hash
            } else {
                continue;
            }
        } else {
            let tmp = "temporary";
            let hash = Object::blob_from_file(&entry.path())
                .context("open blob input file")?
                .write(std::fs::File::create(tmp).context("write blog object to temporary file")?)
                .context("stream file into blob")?;
            let hash_hex = hex::encode(hash);
            std::fs::create_dir_all(format!(".git/objects/{}/", &hash_hex[..2]))
                .context("create subdir of .git/objects")?;
            std::fs::rename(
                tmp,
                format!(".git/objects/{}/{}", &hash_hex[..2], &hash_hex[2..]),
            )
            .context("move blob file into .git/objects")?;
            // let mut hash = [0; 20];
            // hash.copy_from_slice(hash_hex.as_bytes());
            hash
        };
        // {mode} {filename}\0{20字节二进制SHA-1}
        tree_object.extend_from_slice(mode.as_bytes());
        tree_object.push(b' ');
        tree_object.extend(file_name.as_encoded_bytes());
        tree_object.push(b'\0');
        tree_object.extend(hash);
    }
    if tree_object.is_empty() {
        Ok(None)
    } else {
        // write the tree object to the objects directory
        Ok(Some(
            Object {
                kind: Kind::Tree,
                expected_size: tree_object.len() as u64,
                reader: Cursor::new(tree_object),
            }
            .write_to_objects()
            .context("write tree object to .git/objects")?,
        ))
    }
}

pub(crate) fn invoke() -> Result<()> {
    let Some(hash) = write_tree_for(Path::new(".")).context("construct root tree object")? else {
        anyhow::bail!("asked to make tree object for empty directory");
    };
    println!("{}", hex::encode(hash));
    Ok(())
}
