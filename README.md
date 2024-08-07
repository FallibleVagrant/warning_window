# warning_window

![demo](./demo.gif)

> [!WARNING]
> This is a hobby project that is mostly finished. It won't be receiving new features.

Description pending...

## Dependencies
This project uses Raylib, as well as my custom Rust bindings for it. Both are included as submodules of the repo.
[https://github.com/raysan5/raylib](https://github.com/raysan5/raylib)
[https://github.com/falliblevagrant/adhocrays](https://github.com/falliblevagrant/adhocrays)

Building on Windows requires modifying the build script of adhocrays. I don't have a Windows machine to test on.

## Build from Source
To clone the repository, run:
```
git clone https://github.com/FallibleVagrant/warning_window --recurse-submodules --shallow-submodules
```
Raylib is set as a submodule of adhocrays, which is a submodule of this repo.

The command downloads a specific commit of both, to a depth of one.

Alternatively, you may clone this repo as usual and run:
```
git submodule update --init --depth 1 --recursive
```
It achieves the same effect.

To run the server component:
```
cd ww
cargo run
```

To run a client:
```
cd client
cargo run
```

## License
warning_window is licensed under the GPL, see LICENSE for more information.
adhocrays and raylib are available under zlib licenses.
