# krilla-svg2pdf

A CLI for `krilla-svg` that converts an SVG file to a PDF file.

It is also the successor to [`typst/svg2pdf`](https://github.com/typst/svg2pdf).

## Usage

You can convert an SVG file to a PDF file by running commands like these:

```bash
krilla-svg2pdf /path/to/your.svg
```

This will create a PDF file named `your.pdf` in the same directory as the SVG file.

For more customization, you can specify the output file and other options:

```bash
$ krilla-svg2pdf -h
The command line interface for krilla-svg.

Usage: krilla-svg2pdf [OPTIONS] [INPUT] [OUTPUT] [COMMAND]

Commands:
  fonts  Lists all discovered fonts in system
  help   Print this message or the help of the given subcommand(s)

Arguments:
  [INPUT]   Path to read SVG file from
  [OUTPUT]  Path to write PDF file to

Options:
      --dpi <DPI>                    The number of SVG pixels per PDF points [default: 72]
  -t, --text-to-paths                Whether text should be converted to paths before embedding into the PDF
      --raster-scale <RASTER_SCALE>  How much raster images of rasterized effects should be scaled up [default: 1.5]
      --font-path <DIR>              Adds additional directories to search for fonts
      --ignore-system-fonts          Ensures system fonts won't be used, unless explicitly included via `--font-path`
  -h, --help                         Print help (see more with '--help')
  -V, --version                      Print version
```

## License

This crate is dual-licensed under the MIT and Apache 2.0 licenses.