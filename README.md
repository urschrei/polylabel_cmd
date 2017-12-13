# Polylabel_Cmd
…is the command-line version of [Polylabel](https://github.com/urschrei/polylabel-rs). Install it using `cargo install polylabel_cmd`.  
This gives you the `polylabel` command.

## Use
Polylabel takes one mandatory argument: a path to a valid GeoJSON file, containing a `FeatureCollection` of valid polygons. It also accepts an optional `-t` or `--tolerance` switch, allowing you to fine-tune the tolerance from the default `1.0`. Smaller tolerances take longer to calculate.

## Speed
Polylabel is fast. Polygons are processed in parallel, using [Rayon](https://github.com/rayon-rs/rayon).

## Binaries
Will be available when I set up CI.

## License
[MIT](license.txt)
