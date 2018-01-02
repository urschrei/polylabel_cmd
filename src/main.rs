use std::fs::File;
use std::io::prelude::*;
use std::io::Error as IoErr;
use std::mem::replace;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
use geo::{LineString, MultiPoint, MultiPolygon, Point, Polygon};

extern crate geojson;
use geojson::{Error as GjErr, Feature, FeatureCollection, GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate serde_json;
use serde_json::{to_string_pretty, Map};

extern crate polylabel;
use polylabel::polylabel;

extern crate rayon;
use rayon::prelude::*;

extern crate failure;

#[macro_use]
extern crate failure_derive;

#[derive(Fail, Debug)]
enum PolylabelError {
    #[fail(display = "IO error: {}", _0)] IoError(#[cause] IoErr),
    #[fail(display = "GeoJSON deserialisation error: {}. Is your GeoJSON valid?", _0)]
    GeojsonError(#[cause] GjErr),
}

impl From<IoErr> for PolylabelError {
    fn from(err: IoErr) -> PolylabelError {
        PolylabelError::IoError(err)
    }
}

impl From<GjErr> for PolylabelError {
    fn from(err: GjErr) -> PolylabelError {
        PolylabelError::GeojsonError(err)
    }
}

/// Attempt to open a file, read it, and parse it into `GeoJSON`
fn open_and_parse(p: &str) -> Result<GeoJson, PolylabelError> {
    let mut f = File::open(p)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents.parse::<GeoJson>()?)
}

/// Process top-level GeoJSON items
fn process_geojson(gj: &mut GeoJson, tolerance: &f32) {
    match *gj {
        GeoJson::FeatureCollection(ref mut collection) => collection.features
            // Iterate in parallel when appropriate
            .par_iter_mut()
            // Only pass on non-empty geometries, doing so by reference
            .filter_map(|feature| feature.geometry.as_mut())
            .for_each(|geometry| match_geometry(geometry, tolerance)),
        GeoJson::Feature(ref mut feature) => {
            if let Some(ref mut geometry) = feature.geometry {
                match_geometry(geometry, tolerance)
            }
        }
        GeoJson::Geometry(ref mut geometry) => match_geometry(geometry, tolerance),
    }
}

/// Process GeoJSON geometries
fn match_geometry(geom: &mut Geometry, tolerance: &f32) {
    match geom.value {
        Value::Polygon(_) => label(Some(geom), tolerance),
        Value::MultiPolygon(_) => {
            label(Some(geom), tolerance)
        }
        Value::GeometryCollection(ref mut collection) => {
            // GeometryCollections contain other Geometry types, and can nest
            // we deal with this by recursively processing each geometry
            collection
                .par_iter_mut()
                .for_each(|geometry| match_geometry(geometry, tolerance))
        }
        // Point, LineString, and their Multi– counterparts
        _ => (),
    }
}

/// Generate a label position for a (Multi)Polygon
fn label(geom: Option<&mut Geometry>, tolerance: &f32) {
    if let Some(gmt) = geom {
        // construct a fake empty Polygon – this doesn't allocate
        // TODO if Geo geometry validation lands, this will fail
        let v1: Vec<Point<f32>> = Vec::new();
        let ls2 = Vec::new();
        let fake_polygon: Polygon<f32> = Polygon::new(LineString::from(v1), ls2);
        // convert it into a Value, and swap it for our actual (Multi)Polygon
        gmt.value = match gmt.value {
            Value::Polygon(_) => {
                let mut intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let mut geo_type: Polygon<f32> = intermediate
                    .try_into()
                    .expect("Failed to convert a Polygon");
                // generate a label position Point for it, and put it back
                Value::from(&polylabel(&geo_type, tolerance))
            }
            Value::MultiPolygon(_) => {
                let mut intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let mut geo_type: MultiPolygon<f32> = intermediate
                    .try_into()
                    .expect("Failed to convert a MultiPolygon");
                // we allocate here – can we avoid it? idk
                let mp = MultiPoint(
                    geo_type
                        .0
                        .par_iter()
                        .map(|polygon| polylabel(polygon, tolerance))
                        .collect(),
                );
                // move label positions into geometry
                Value::from(&mp)
            }
            _ => {
                let mut intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                intermediate
            }
        }
    }
}

fn main() {
    let command_params = App::new("polylabel")
       .version(&crate_version!()[..])
       .author("Stephan Hügel <urschrei@gmail.com>")
       .about("Find optimum label positions for polygons")
       .args_from_usage("-t --tolerance=[TOLERANCE] 'Set a tolerance for finding \
        the label position. Defaults to 0.001'")
       .arg(Arg::with_name("pretty")
                .help("Pretty-print GeoJSON output")
                .short("p")
                .long("pretty"))
       .arg(Arg::with_name("GEOJSON")
                .help("GeoJSON with a FeatureCollection containing one or more (Multi)Polygons, \
                 or a Feature containing a Multi(Polygon), or a Geometry that is a (Multi)Polygon, \
                 or a GeometryCollection containing (Multi)Polygons.")
                .index(1)
                .required(true))
       .get_matches();

    let tolerance = value_t!(command_params.value_of("TOLERANCE"), f32).unwrap_or(0.001);
    let poly = value_t!(command_params.value_of("GEOJSON"), String).unwrap();
    let pprint = command_params.is_present("pretty");
    let res = open_and_parse(&poly);
    match res {
        Err(e) => println!("{}", e),
        Ok(mut gj) => {
            process_geojson(&mut gj, &tolerance);
            let to_print = if !pprint {
                gj.to_string()
            } else {
                to_string_pretty(&gj).unwrap()
            };
            println!("{}", to_print);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{label_for_geojson, open_and_parse};
    use geojson::GeoJson;
    #[test]
    /// Can a nested GeometryCollection be parsed?
    fn test_nested_geometrycollection() {
        let raw_gj = r#"
        {
          "features": [
            {
              "geometry": {
                "geometries": [
                  {
                    "coordinates": [
                      2.8125,
                      -4.0625
                    ],
                    "type": "Point"
                  },
                  {
                    "coordinates": [
                      [
                        4.375,
                        -6.125
                      ],
                      [
                        6.890625,
                        -8.015625
                      ]
                    ],
                    "type": "MultiPoint"
                  },
                  {
                    "geometries": [
                      {
                        "coordinates": [
                          -3.248626708984375,
                          -4.188140869140625
                        ],
                        "type": "Point"
                      }
                    ],
                    "type": "GeometryCollection"
                  }
                ],
                "type": "GeometryCollection"
              },
              "properties": {},
              "type": "Feature"
            }
          ],
          "type": "FeatureCollection"
        }
        "#;
        let correct = raw_gj.parse::<GeoJson>().unwrap();
        let gj = open_and_parse(&"geojson/geometrycollection_nested.geojson");
        let fc = label_for_geojson(gj.unwrap(), &0.001).unwrap();
        assert_eq!(GeoJson::from(fc), correct);
    }
}
