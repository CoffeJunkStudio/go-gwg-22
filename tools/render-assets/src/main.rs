use std::time::Duration;
use std::{path::PathBuf, fs};
use std::process::Command;
use std::collections::HashMap;

use indicatif::{ProgressBar, ProgressStyle};

const RENDER_ASSET_SCRIPT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/render-asset.py"));

const fn default_asset_width() -> u32 {
	256
}
const fn default_asset_n_frames() -> u32 {
	32
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
struct AssetConfig {
	#[serde(default = "default_asset_width")]
	width: u32,

	height: Option<u32>,

	#[serde(default = "default_asset_n_frames")]
	n_frames: u32,

	output: Option<PathBuf>,
}

#[cfg(target_family = "windows")]
fn blender_exe() -> PathBuf {
	PathBuf::from("C:")
		.join("Program Files")
		.join("Blender Foundation")
		.join("Blender 3.0")
		.join("blender.exe")
}

#[cfg(not(target_family = "windows"))]
fn blender_exe() -> PathBuf {
	PathBuf::from("blender")
}

fn main() {
	let out_dir = PathBuf::from("assets").join("rendered");

	let render_config_path = PathBuf::from("render_assets.toml");
	let render_config_dir = render_config_path.parent().unwrap();

	let render_config_str = fs::read_to_string(&render_config_path).unwrap();
	let render_config: HashMap<PathBuf, HashMap<String, AssetConfig>> =
		toml::from_str(&render_config_str).unwrap();
	
	let progress = ProgressBar::new(render_config.values().flat_map(|v| v.iter()).count() as u64);
	progress.set_style(ProgressStyle::with_template("{spinner:.green} {msg} [{wide_bar}] {pos}/{len} {percent}%").unwrap()
		.progress_chars("=> "));
	progress.enable_steady_tick(Duration::from_millis(200));
	progress.inc(0);

	for (blend_file_name, assets_config) in render_config {
		let blend_file_path = render_config_dir.join(&blend_file_name);
		for (asset_name, asset_config) in assets_config {
			let out_filename = asset_config
				.output
				.unwrap_or_else(|| PathBuf::from(format!("{}.png", &asset_name)));
			
			progress.set_message(format!("Rendering {} | {} > {}", blend_file_path.file_name().unwrap().to_string_lossy(), asset_name, out_filename.file_name().unwrap().to_string_lossy()));
			let out_path = out_dir.join(out_filename);

			let blender_out = Command::new(blender_exe())
				.arg("--background")
				.arg(&blend_file_path)
				.arg("--python-expr")
				.arg(RENDER_ASSET_SCRIPT)
				.arg("--")
				.arg("--output")
				.arg(out_path)
				.arg("--object-name")
				.arg(&asset_name)
				.arg("--width")
				.arg(asset_config.width.to_string())
				.arg("--n-frames")
				.arg(asset_config.n_frames.to_string())
				.output()
				.unwrap_or_else(|err| panic!("Failed to render {}: {err}", &asset_name));

			if !blender_out.status.success() {
				eprintln!("Failed to render {asset_name}:");
				eprintln!("-- blender stdout:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stdout));
				eprintln!("-- blender stderr:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stderr));
				panic!("Rendering failed")
			}

			progress.inc(1);
		}
	}
}
