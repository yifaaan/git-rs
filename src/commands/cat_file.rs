use std::io::Write;

use crate::{
    objects::{object_find, object_read},
    repository::repo_find,
    ObjectType,
};
use anyhow::Result;

pub(crate) fn cmd_cat_file(tp: ObjectType, obj: String) -> Result<()> {
    let repo = repo_find(".", true)?;
    let obj = object_read(&repo, &object_find(&repo, obj, tp)?)?;
    std::io::stdout().write_all(&obj.serialize())?;
    Ok(())
}
