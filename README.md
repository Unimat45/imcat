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
It automatically resizes to the width of your terminal, using proper sampling kernels.

imcat also works on the latest version of Windows 10.

## Usage

```
$ imcat file1 [file2 .. fileN]
```

If you want to blend the image with the terminal background, then you need to specify the background color of your terminal. For instance:

```
$ export IMCATBG="#dad9cc"
```

## Todo

* Handle alpha-transparency. Requires reading the terminal background colour, somehow. Tricky.

## Done

* Correct for aspect ratio of terminal font.

The output width follows your terminal. To override it (for instance when piping into a file), set `IMCAT_WIDTH`:

```
$ IMCAT_WIDTH=120 imcat file1 > preview.txt
```

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

## License

* [CC0](https://creativecommons.org/publicdomain/zero/1.0/)

## Authors

[Bram Stolk](http://stolk.org).

Image decoding is done by the [image](https://crates.io/crates/image) crate. The original C version used [stb_image.h](http://nothings.org/stb_image.h) by Sean Barrett et al.

## Acknowledgements

A shout out to [Frogtoss](http://github.com/mlabbe) for the idea and help. Thanks!

## Gallery

![Sample use](images/sampledesktop.png "Sample use.")

