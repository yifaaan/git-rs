use std::path::Path;

use anyhow::Result;

use crate::repository::repo_create;

pub(crate) fn cmd_init<P: AsRef<Path>>(path: Option<P>) -> Result<()> {
    if let Some(p) = path {
        repo_create(p)?;
    } else {
        repo_create(".")?;
    }
    Ok(())
}
