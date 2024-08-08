# warning_window

![demo](./demo.gif)

> [!WARNING]
> This is a hobby project that is mostly finished. It won't be receiving new features.

warning_window is largely an experiment in writing a networked application with a binary protocol. Its functionality is similar to the `notify-send` command on most Linux distributions.

The server component receives messages over the network from any number of clients. There are no controls in place to handle malicious clients, though, so don't expose the server to the public Internet unless it's through a tunnel that requires authorization first. If you are curious about the specifics of the protocol, there is a little documentation in the source code.

The project was inspired by the small corner widget I implemented for [connwatch](https://github.com/falliblevagrant/connwatch), as well as [this animation by vewn](https://www.youtube.com/watch?v=KJNWlMiL1zM&t=52) ([invidious link](https://iv.melmac.space/watch?v=KJNWlMiL1zM&t=52)).

This repository contains the server (the "ww" folder), an interactive client, and a convenience API for sending network requests to a server.

## Dependencies
This project uses Raylib, as well as my custom Rust bindings for it. Both are included as submodules of this repo.

You may find Raylib [here](https://github.com/raysan5/raylib).

Adhocrays, the custom Rust bindings, are [here](https://github.com/falliblevagrant/adhocrays).

Building on Windows requires modifying the build script of adhocrays, since I don't have a machine to test on.

Lastly, the server is a TUI and so uses [crossterm](https://github.com/crossterm-rs/crossterm), which cargo will download automatically.

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

Once the repo is downloaded, run the server component with:
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
warning_window is licensed under GPLv2, see LICENSE for more information.

adhocrays and raylib are available under zlib licenses.
