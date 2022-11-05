use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
	package_assets();

	// Notice that because, the `package_assets` emits a `rerun-if-changed`,
	// this build script as whole is only run if the "assets" folder changed,
	// which can lead to an out-dated built info.
	//
	// Therefore, release builds should be preceded with a `cargo clean`.
	built_info();
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
