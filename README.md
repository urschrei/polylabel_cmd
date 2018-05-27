[![Linux / macOS Build Status](https://travis-ci.org/urschrei/polylabel_cmd.svg?branch=master)](https://travis-ci.org/urschrei/polylabel_cmd) [![Windows Build status](https://ci.appveyor.com/api/projects/status/hfmd4lio8hqc4ig8/branch/master?svg=true)](https://ci.appveyor.com/project/urschrei/polylabel-cmd/branch/master)
 [![Crates Link](https://img.shields.io/crates/v/polylabel_cmd.svg)](https://crates.io/crates/polylabel_cmd)
# `polylabel_cmd`
…is the command-line version of [Polylabel](https://github.com/urschrei/polylabel-rs). Install it using `cargo install polylabel_cmd`, or download a [binary](#binaries) and put it on your $PATH.  
This provides the `polylabel` command.

## Use
Polylabel takes one mandatory argument: a file containing valid GeoJSON containing Polygons and / or MultiPolygons to be labelled. They can be included as a `Feature,` or a `Geometry`, or a`FeatureCollection` or `GeometryCollection` – you may also mix the two geometries in a `FeatureCollection` or `GeometryCollection`.

- Processing of nested `GeometryCollection`s is supported, [but you shouldn't be using those](https://tools.ietf.org/html/rfc7946#section-3.1.8)
- Empty geometries or collections will be left unaltered
- Non Multi(Polygon) geometries will be left unaltered
- All properties will be left unaltered

You may also pass:
- `-t` or `--tolerance`, allowing you to fine-tune the tolerance from the default `0.001`. Smaller tolerances take longer to calculate
- `-p` or `--pretty`, which will pretty-print the GeoJSON output
- `-s` or `--stats-only`, which will output the number of labelled polygons, but will *not* output GeoJSON.

Irrespective of input, output is a GeoJSON `FeatureCollection`. Its contents depend on the input geometry:
- `Polygon`: The `FeatureCollection` contains `Point` `Feature`s
- `MultiPolygon`: The `FeatureCollection` contains `MultiPoint` `Feature`s
- `GeometryCollection`: The `FeatureCollection` contains `GeometryCollection` `Feature`s whose collection members are `Point`s or `MultiPoint`s
- Other geometries are included in one the above outputs, but are otherwise left unaltered.

Output features retain the order of input features / geometries, and input feature properties are mapped to output features where they exist.

### Accuracy
Depending upon the dimensions of your polygon(s), you may require a higher tolerance (i.e. a smaller number) than the default. See [here](https://gis.stackexchange.com/questions/8650/measuring-accuracy-of-latitude-and-longitude/8674#8674) for some guidance on the accuracy provided by each decimal place. The GeoJSON spec _recommends_ a maximum of six decimal places, which provides accuracy around 10cm, which translates to `-t 0.000001`, which should be sufficient for applications which don't require survey-quality accuracy.

### Progress
If you aren't piping the output of the command to a file, `polylabel` will display progress of the parsing and labelling steps in the terminal, as well as a final count of the labelled polygons.

## Validity
While the structure of the input GeoJSON is validated, individual geometries are *not* validated in the DE-9IM sense. If they self-intersect, have open rings etc., results are not guaranteed to be correct.

## Speed
It runs ~10x faster than the [NPM](https://www.npmjs.com/package/geojson-polygon-labels) package. Polygons are processed in parallel, using [Rayon](https://github.com/rayon-rs/rayon). Note that higher tolerances will result in slower processing.

## Binaries
Pre-built binaries are available from [releases](https://github.com/urschrei/polylabel_cmd/releases/latest). Binaries are available for:
- macOS (x86_64)
- Linux (x86_64)
- Windows (x86_64 and i686)

## License
[MIT](license.txt)
