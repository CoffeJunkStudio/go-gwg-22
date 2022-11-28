Plenty of Fish in the Sea
=========================

> Our Game-Off 2022 game jam participation
>
> Theme: _Cliché_

Our game is written in Rust with the [good-web-game] crate.
It can be compiled as an HTML 5 game, natively to Linux and Windows, and for
Android.

You can see a preview of the current `main` branch at our [Github Page][preview].
Or get the "stable" version from our [itch.io page][itchio].

[good-web-game]: https://crates.io/crates/good-web-game
[preview]: https://coffejunkstudio.github.io/go-gwg-22/
[itchio]: https://coffejunkstudio.itch.io/plenty-of-fish?secret=fHOi58o4GNbMuuYhHvLTumPb1ZU

Game Play
---------

_We are currently still implementing the core game._

The core game play is about manoeuvering a sailing ship to catch fish from the sea.


### Controls

| Key       | Function |
|-----------|----------|
| Up \| `W` | Hoist the sails |
|Down\| `S` | Take in the sails |
|Left\| `A` | Turn left |
|Right\| `D`| Turn right |
| `E`       | Sell fish (at a harbor) |
| `R`       | Upgrade Sail (at a harbor) |
| `F`       | Upgrade Hull (at a harbor) |
| `1`       | Toggle sounds |
| `2`       | Toggle music |
| PgUp      | Zoom in |
| PgDown    | Zoom out |
| Backspace | Reset zoom |
| `ESC`     | Quit |
| `F11`     | Enter full screen |


Prerequisites
-------------

In order to build this game from source, you need a decent Rust compiler and the
Cargo package manager, you can get both from [here][rust-get-started].

Also since we put the assets into a sub-module, you need to initialize it first:

```sh
git submodule update --init
```

### Native

On Linux you need at least the following additional libraries:

- `libasound2-dev`
- `libudev-dev`
- `blender`

_This list omits "default" libraries._

The native requirements for other OSes are undetermined yet.


#### Cross-compiling to Windows

In order to cross-compile from Linux to windows you need a C cross-compiler,
best to use GCC:

- `gcc-mingw-w64-x86-64`

And of course the Rust target:

```sh
rustup target add x86_64-pc-windows-gnu
```

You might also want to install `wine` for testing the results.


### Web

For building a WASM assembly, you need the WASM32 target, if you have
`rustup` you can easily add WASM32 support via:

```sh
rustup target add wasm32-unknown-unknown
```

Additionally, to easily serve such a WASM assembly, you might like some simple
HTTP server that dose just that, such as [simple-http-server], which you can
install via Cargo:

```sh
cargo install simple-http-server
```


[simple-http-server]: https://crates.io/crates/simple-http-server
[rust-get-started]: https://www.rust-lang.org/learn/get-started




Running
-------


### Native

To run this game natively, just execute the following:

```sh
cargo run
```


### Cross-compiling to Windows

```sh
cargo build --target x86_64-pc-windows-gnu --release
```

If you have `wine` you can run the `exe` file via:

```sh
wine target/x86_64-pc-windows-gnu/release/gwg-prep.exe
```


### Web

To run this game via WASM in a browser you first have to build the WASM assembly via:

```sh
./build-web.sh
```

And then you can serve the generated `target/web-pkg` with whatever plain HTTP server you like, e.g.:

```sh
simple-http-server --index --nocache target/web-pkg
```

And head to <http://localhost:8000>



#### `wasm-bindgen` compatibility

Despite the fact that this project transitively depends on `wasm-bindgen`, we don't use it as a build tool.
I.e. we don't use a `wasm-bindgen` JS-glue, but instead the Macroquad's JS-glue (that also contains the miniquad WebGL stuff).
For some reason it all just happen to work, let's see when it breaks.

### Android

In order to build for Android, we use a `cargo-apk` variation from notfl3,
which is just a Docker image with all the build tools. Well actually, we
additionally need the `libz` and `blender`.

Anyway, you can build the game for Android via using the following Docker container:

```sh
docker run -it --rm -v $(pwd):/root/src -w /root/src notfl3/cargo-apk
```

And executing the following sequence of command therein

```sh
apt update
apt install libz-dev blender
cargo quad-apk build --release
```


