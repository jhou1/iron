use crate::db::Database;
use std::path::{Path, PathBuf};

pub fn export_to_json(_db: &Database, _path: Option<PathBuf>) -> anyhow::Result<()> {
    todo!()
}

pub fn import_from_json(_db: &Database, _path: &Path) -> anyhow::Result<usize> {
    todo!()
}
