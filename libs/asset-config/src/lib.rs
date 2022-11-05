use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AssetConfig {
	pub file: HashMap<PathBuf, HashMap<String, SingleAssetConfig>>,
	pub sail: HashMap<String, SailParams>,
}

const fn default_asset_width() -> u32 {
	256
}
const fn default_asset_z_frames() -> u32 {
	1
}
const fn default_asset_x_frames() -> u32 {
	1
}

#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SingleAssetConfig {
	#[serde(default = "default_asset_width")]
	pub width: u32,

	pub height: Option<u32>,

	#[serde(default = "default_asset_z_frames")]
	pub z_frames: u32,

	#[serde(default = "default_asset_x_frames")]
	pub x_frames: u32,

	pub object: String,
	pub output: Option<PathBuf>,

	/// Mass in kilograms
	pub mass: Option<u32>,

	/// Size of the object in blender units
	pub optical_size: f32,

	/// Size of the object in the game world in meters
	pub logical_size: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SailParams {
	/// Offset of this sail's mast along the x-axis in blender units (0 = center of ship)
	pub mast_offset: f32,

	/// Area of this sail in square meters
	pub area: f32,

	pub reefing_stages: Vec<String>,
}
