#!/bin/bash

# Build the `plenty-of-fish-in-the-sea` web-app (aka `wasm32-unknown-unknown`)
# start with `--release` for release mode
#
# Use the environment variable `FEATURES` to enable any features (use comma to
# separate multiple)

# Notice this is a Macroquad special!
# Appearently, also works with Miniquad.

set -e

target_name="debug"
build_flags=""

while [ $# -ge 1 ]
do
	if [ "$1" == "--release" ]
	then
		shift
		target_name="release"
		build_flags="--release"
	else
		echo "Error: invalid argument"
		exit 1
	fi
done


APP_NAME="plenty-of-fish-in-the-sea"

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
TARGET_DIR="$SCRIPT_DIR/../target"
OUT_DIR="$TARGET_DIR/web-pkg"
ARCH="wasm32-unknown-unknown"
FEAT=""

if [ -n "$FEATURES" ]
then
	FEAT="--features $FEATURES"
fi

if [[ $target_name = "release" ]]
then
	echo "Cleaning..."
	# First clean that target to ensure that we get a fresh build
	cargo clean --package "$APP_NAME" --target $ARCH $build_flags

	rm -rf "$OUT_DIR"
fi

echo "Create output dir '$OUT_DIR'"
mkdir -p "$OUT_DIR"

# Build wasm binary and binding JS
echo "Build WASM in $target_name mode"
# to make even smaller we could use nightly compiler features such as:
#   -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort
# But the latter, is a bit ify
cargo build --package "$APP_NAME" --target $ARCH $build_flags $FEAT

echo "Copy wasm binary"
cp "$TARGET_DIR/$ARCH/$target_name/$APP_NAME.wasm" "$OUT_DIR"

# Copy web assets
echo "Copy assets"
cp -r "$SCRIPT_DIR/static/"* "$OUT_DIR"

echo "Done"

# Start server:
# simple-http-server --index --nocache target/web-pkg/
