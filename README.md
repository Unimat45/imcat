# imcat

Preview any size image in a terminal window.


![imcat tiger](images/imcat_tiger.png "imcat tiger")

# Platform

Supports Linux, MacOS and Microsoft Windows 10 terminals.


## Introduction

Do you ever find yourself logged into a server, over SSH, but there is no X11 connection.
And you want to see the contents of an image?

imcat to the rescue!

imcat is a 24-bit image viewer that uses ANSI terminal colours to display PNG, JPEG, WebP, GIF, TIFF, BMP, ICO, TGA, QOI, HDR, PNM, DDS and farbfeld images.
It automatically resizes to the width of your terminal, averaging every pixel so fine detail doesn't turn into noise, and rotates photos according to their EXIF orientation.

imcat also works on the latest version of Windows 10.

## Usage

```
$ imcat [OPTIONS] file1 [file2 .. fileN]
```

| Option | Environment variable | Meaning |
|---|---|---|
| `-w`, `--width COLUMNS` | `IMCAT_WIDTH` | Maximum output width. Defaults to the terminal width (80 if there is no terminal). Images are never enlarged. |
| `-b`, `--bg #RRGGBB` | `IMCATBG` | Your terminal's background colour. Transparent areas are blended into it instead of black. |
| `-h`, `--help` | | Show help. |
| `-V`, `--version` | | Show the version. |

Options given on the command line override the environment variables. For example, to blend with a light terminal background in every session:

```
$ export IMCATBG="#dad9cc"
```

or to save a preview at a fixed width:

```
$ imcat -w 120 file1 > preview.txt
```

imcat exits with status 1 if any image could not be loaded, and 2 for invalid options.

## Done

* Correct for aspect ratio of terminal font.
* Alpha transparency, blended with the terminal background colour given by `--bg` or `IMCATBG`.

## Building

imcat is written in Rust. With a [Rust toolchain](https://rustup.rs) installed, the same steps work on Linux, macOS and Windows:

```
$ cargo install --path .      # builds and installs imcat into ~/.cargo/bin
```

or, to just build the binary:

```
$ cargo build --release       # produces target/release/imcat (imcat.exe on Windows)
```

The release binary is self-contained, so you can copy it to another machine with the same OS and architecture. A man page is provided in `imcat.1`.

To run the tests:

```
$ cargo test
```

Some tests compare output with saved snapshots in `tests/snapshots/`. If you change the output on purpose, check it and then refresh them with `IMCAT_BLESS=1 cargo test`.

## License

* [CC0](https://creativecommons.org/publicdomain/zero/1.0/)

## Authors

[Bram Stolk](http://stolk.org).

Image decoding is done by the [image](https://crates.io/crates/image) crate. The original C version used [stb_image.h](http://nothings.org/stb_image.h) by Sean Barrett et al.

## Acknowledgements

A shout out to [Frogtoss](http://github.com/mlabbe) for the idea and help. Thanks!

## Gallery

![Sample use](images/sampledesktop.png "Sample use.")

