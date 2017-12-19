# Polylabel_Cmd
…is the command-line version of [Polylabel](https://github.com/urschrei/polylabel-rs). Install it using `cargo install polylabel_cmd`.  
This gives you the `polylabel` command.

## Use
Polylabel takes one mandatory argument: valid GeoJSON, containing any 1 of:

- a `FeatureCollection` containing `Feature`s which are valid:
    - `Polygon`s or
    - `MultiPolygon`s or
    - `GeometryCollection`s containing either or both of the above, or 
- a `Feature` containing a valid:
    - `Polygon` or
    - `MultiPolygon` or
    - `GeometryCollection` containing either or both of the above.
- a `Geometry` which is a valid
    - `Polygon` or
    - `MultiPolygon` or
    - `GeometryCollection` containing either or both of the above.
- Processing of nested `GeometryCollection`s is supported, [but you shouldn't be using those](https://tools.ietf.org/html/rfc7946#section-3.1.8)
- Non-(`Multi`)`Polygon` geometries are **stripped** from any output.  

It also accepts an optional `-t` or `--tolerance` switch, allowing you to fine-tune the tolerance from the default `0.001`. Smaller tolerances take longer to calculate.  

A  `-p` or `--pretty` flag may be set, which will pretty-print the GeoJSON output.   

Irrespective of input, successful output is a GeoJSON `FeatureCollection`. Its contents depend on the input geometry:
- `Polygon`: The `FeatureCollection` contains `Point` `Feature`s
- `MultiPolygon`: The `FeatureCollection` contains `MultiPoint` `Feature`s
- `GeometryCollection`: The `FeatureCollection` contains `GeometryCollection` `Feature`s whose collection members are `Point`s or `MultiPoint`s.

Output features retain the order of input features / geometries, and input feature properties are mapped to output features where they exist.

## Validity
Input geometries are *not* validated. Results from invalid input geometries may be incorrect.

## Speed
Polylabel is fast. Polygons are processed in parallel, using [Rayon](https://github.com/rayon-rs/rayon).

## Binaries
Will be available when I set up CI.

## License
[MIT](license.txt)
