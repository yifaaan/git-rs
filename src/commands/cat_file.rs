use crate::objects::{Kind, Object};
use anyhow::{Context, Result};

pub fn invoke(pretty_print: bool, object_hash: String) -> Result<()> {
    anyhow::ensure!(pretty_print, "pretty printing is not yet implemented");
    let mut object = Object::read(&object_hash).context("parse out blob object file")?;

    match object.kind {
        Kind::Blob => {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            let n = std::io::copy(&mut object.reader, &mut stdout)
                .context("write .git/objects file to stdout")?;
            anyhow::ensure!(
                n == object.expected_size,
                ".git/objects file was not the expected size (expected: {}, actual: {})",
                object.expected_size,
                n
            );
        }
        _ => anyhow::bail!("don't know how to print {}", object.kind),
    }
    Ok(())
}
