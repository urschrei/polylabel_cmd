[![Linux / macOS Build Status](https://travis-ci.org/urschrei/polylabel_cmd.svg?branch=master)](https://travis-ci.org/urschrei/polylabel_cmd) [![Windows Build status](https://ci.appveyor.com/api/projects/status/hfmd4lio8hqc4ig8/branch/master?svg=true)](https://ci.appveyor.com/project/urschrei/polylabel-cmd/branch/master)
 [![Crates Link](https://img.shields.io/crates/v/polylabel_cmd.svg)](https://crates.io/crates/polylabel_cmd)
# `polylabel_cmd`
…is the command-line version of [Polylabel](https://github.com/urschrei/polylabel-rs). Install it using `cargo install polylabel_cmd`, or download a [binary](#binaries) and put it on your $PATH.  
This gives you the `polylabel` command.

## Use
Polylabel takes one mandatory argument: a file containing valid GeoJSON containing Polygons and / or MultiPolygons to be labelled. They can be included as a `Feature,` or a `Geometry`, or a`FeatureCollection` or `GeometryCollection` – you may also mix the two geometries in a `FeatureCollection` or `GeometryCollection`.

- Processing of nested `GeometryCollection`s is supported, [but you shouldn't be using those](https://tools.ietf.org/html/rfc7946#section-3.1.8)
- Empty geometries or collections will be left unaltered
- Non-(`Multi`)`Polygon` geometries cannot be processed – their presence will halt further processing.

You may also pass:
- `-t` or `--tolerance`, allowing you to fine-tune the tolerance from the default `0.001`. Smaller tolerances take longer to calculate
- `-p` or `--pretty`, which will pretty-print the GeoJSON output
- `-s` or `--stats-only`, which will output the number of labelled polygons, but will *not* output GeoJSON.

Irrespective of input, output is a GeoJSON `FeatureCollection`. Its contents depend on the input geometry:
- `Polygon`: The `FeatureCollection` contains `Point` `Feature`s
- `MultiPolygon`: The `FeatureCollection` contains `MultiPoint` `Feature`s
- `GeometryCollection`: The `FeatureCollection` contains `GeometryCollection` `Feature`s whose collection members are `Point`s or `MultiPoint`s.

Output features retain the order of input features / geometries, and input feature properties are mapped to output features where they exist.

### Progress
If you aren't piping the output of the command to a file, `polylabel` will display progress of the parsing and labelling steps in the terminal, as well as a final count of the labelled polygons.

## Validity
While the structure of the input GeoJSON is validated, individual geometries are *not* validated in the DE-9IM sense. If they self-intersect, have open rings etc., results are not guaranteed to be correct.

## Speed
Polylabel is fast. Polygons are processed in parallel, using [Rayon](https://github.com/rayon-rs/rayon).

## Binaries
Pre-built binaries are available from [releases](https://github.com/urschrei/polylabel_cmd/releases/latest). Binaries are available for:
- macOS (x86_64)
- Linux (x86_64)
- Windows (x86_64 and i686)

## License
[MIT](license.txt)
