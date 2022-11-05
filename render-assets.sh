#!/bin/bash

cd -- "$( dirname -- "${BASH_SOURCE[0]}" )"

cargo run --release --manifest-path tools/render-assets/Cargo.toml
