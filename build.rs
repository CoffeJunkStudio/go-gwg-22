use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

fn main() {
	render_assets();
	package_assets();

	// Notice that because, the `package_assets` emits a `rerun-if-changed`,
	// this build script as whole is only run if the "assets" folder changed,
	// which can lead to an out-dated built info.
	//
	// Therefore, release builds should be preceded with a `cargo clean`.
	built_info();
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

fn render_assets() {
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

	let render_config_path = manifest_dir.join("render_assets.toml");
	let script_path = manifest_dir.join("scripts").join("render-asset.py");

	println!("cargo:rerun-if-changed={}", render_config_path.display());
	println!("cargo:rerun-if-changed={}", script_path.display());

	let render_config_str = fs::read_to_string(&render_config_path).unwrap();
	let render_config: HashMap<PathBuf, HashMap<String, AssetConfig>> =
		toml::from_str(&render_config_str).unwrap();

	for (blend_file_name, assets_config) in render_config {
		let blend_file_path = manifest_dir.join(&blend_file_name);
		for (asset_name, asset_config) in assets_config {
			let out_filename = asset_config
				.output
				.unwrap_or_else(|| PathBuf::from(format!("{}.png", &asset_name)));
			let out_path = out_dir.join("rendered_assets").join(out_filename);

			println!("cargo:rerun-if-changed={}", blend_file_path.display());
			let blender_out = Command::new(blender_exe())
				.arg(&blend_file_path)
				.arg("--background")
				.arg("--python")
				.arg(&script_path)
				.arg("--")
				.arg("--output")
				.arg(out_path)
				.arg("--object-name")
				.arg(&asset_name)
				.output()
				.unwrap_or_else(|err| panic!("Failed to render {}: {err}", &asset_name));

			if !blender_out.status.success() {
				eprintln!("Failed to render {asset_name}:");
				eprintln!("-- blender stdout:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stdout));
				eprintln!("-- blender stderr:");
				eprintln!("{}", String::from_utf8_lossy(&blender_out.stderr));
				panic!()
			}
		}
	}
}

fn built_info() {
	// Write out the built into into the OUT folder.
	built::write_built_file().expect("Failed to acquire build-time information");
}

// Package up all the assets from the `assets` folder
fn package_assets() {
	// We want to rerun this script if anything within the "assets" folder changed
	println!("cargo:rerun-if-changed=assets");

	// We are going to package all that stuff into a tar archive, so we
	// can inline that into our binary, and thus we write it to the OUT folder.
	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
	let tar_path = out_dir.join("assets.tar");

	let tar_file = fs::File::create(tar_path).unwrap();
	let mut tar_builder = tar::Builder::new(tar_file);

	// Move all the asset stuff into the archive
	tar_builder.append_dir_all(".", "assets").unwrap();
	// And write it.
	tar_builder.finish().unwrap();
}
