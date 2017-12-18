# Polylabel_Cmd
…is the command-line version of [Polylabel](https://github.com/urschrei/polylabel-rs). Install it using `cargo install polylabel_cmd`.  
This gives you the `polylabel` command.

## Use
Polylabel takes one mandatory argument: valid GeoJSON, containing any 1 of:

- a `FeatureCollection` containing `Feature`s which are valid `Polygon`s, `MultiPolygon`s, or `GeometryCollection`s containing same.
- a `Feature` containing a valid `Polygon`, `MultiPolygon`, or `GeometryCollection`
- a `Geometry` which is a valid `Polygon`, `MultiPolygon`, or `GeometryCollection`
- Nested `GeometryCollections` are **not** supported.

Any non-(`Multi`)`Polygon` content is ignored.  

It also accepts an optional `-t` or `--tolerance` switch, allowing you to fine-tune the tolerance from the default `0.001`. Smaller tolerances take longer to calculate.   

Irrespective of input, successful output is a GeoJSON `FeatureCollection`. Its contents depend on the input geometry:
- `Polygon`: The `FeatureCollection` contains `Point` `Feature`s
- `MultiPolygon`: The `FeatureCollection` contains `MultiPoint` `Feature`s
- `GeometryCollection`: The `FeatureCollection` contains `GeometryCollection` `Feature`s whose geometries are `Point`s or `MultiPoint`s.

Output features retain the order of input features / geometries, and input feature properties are mapped to output features where they exist.

## Validity
Input geometries are *not* validated. Results from invalid input geometries may be incorrect.

## Speed
Polylabel is fast. Polygons are processed in parallel, using [Rayon](https://github.com/rayon-rs/rayon).

## Binaries
Will be available when I set up CI.

## License
[MIT](license.txt)
