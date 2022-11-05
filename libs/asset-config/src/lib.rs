use std::collections::HashMap;
use std::path::PathBuf;

pub type AssetConfig = HashMap<PathBuf, HashMap<PathBuf, SingleAssetConfig>>;

const fn default_asset_width() -> u32 {
	256
}
const fn default_asset_n_frames() -> u32 {
	32
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SingleAssetConfig {
	#[serde(default = "default_asset_width")]
	pub width: u32,

	pub height: Option<u32>,

	#[serde(default = "default_asset_n_frames")]
	pub n_frames: u32,

	pub object: String,
}
