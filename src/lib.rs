use std::path::Path;
use crate::mapping::MappingLoader;
use crate::tiny_v2::TinyV2Mapping;

pub mod mapping;
pub mod tiny_v2;

#[deprecated(since = "0.1.0", note = "Please use `TinyV2Mapping::load` instead")]
pub fn parse_tiny_v2(file_path: &Path) -> anyhow::Result<TinyV2Mapping> {
    let mapping = TinyV2Mapping::load(file_path)?;
    Ok(mapping)
}
