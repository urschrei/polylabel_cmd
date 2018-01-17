use std::fs::File;
use std::io::prelude::*;
use std::io::Error as IoErr;
use std::mem::replace;
use std::process::exit;
use std::sync::atomic::{AtomicIsize, Ordering};

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

extern crate console;
use console::{style, user_attended};

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

/// Process top-level `GeoJSON` items
fn process_geojson(gj: &mut GeoJson, tolerance: &f32, ctr: &AtomicIsize) {
    match *gj {
        GeoJson::FeatureCollection(ref mut collection) => collection.features
            .par_iter_mut()
            // Only pass on non-empty geometries, doing so by reference
            .filter_map(|feature| feature.geometry.as_mut())
            .for_each(|geometry| label_geometry(geometry, tolerance, ctr)),
        GeoJson::Feature(ref mut feature) => {
            if let Some(ref mut geometry) = feature.geometry {
                label_geometry(geometry, tolerance, ctr)
            }
        }
        GeoJson::Geometry(ref mut geometry) => label_geometry(geometry, tolerance, ctr),
    }
}

/// Process `GeoJSON` geometries
fn label_geometry(geom: &mut Geometry, tolerance: &f32, ctr: &AtomicIsize) {
    match geom.value {
        Value::Polygon(_) | Value::MultiPolygon(_) => label_value(Some(geom), tolerance, ctr),
        Value::GeometryCollection(ref mut collection) => {
            // GeometryCollections contain other Geometry types, and can nest
            // we deal with this by recursively processing each geometry
            collection
                .par_iter_mut()
                .for_each(|geometry| label_geometry(geometry, tolerance, ctr))
        }
        // Point, LineString, and their Multi– counterparts
        // bail out early
        _ => {
            println!("Non-Polygon or MultiPolygon geometries detected. Please remove these before retrying.");
            exit(1)
        }
    }
}

/// Generate a label position for a (Multi)Polygon Value
fn label_value(geom: Option<&mut Geometry>, tolerance: &f32, ctr: &AtomicIsize) {
    if let Some(gmt) = geom {
        // construct a fake empty Polygon – this doesn't allocate
        // TODO if Geo geometry validation lands, this will fail
        let v1: Vec<Point<f32>> = Vec::new();
        let ls2 = Vec::new();
        let fake_polygon: Polygon<f32> = Polygon::new(LineString::from(v1), ls2);
        // convert it into a Value, and swap it for our actual (Multi)Polygon
        gmt.value = match gmt.value {
            Value::Polygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let geo_type: Polygon<f32> = intermediate
                    .try_into()
                    .expect("Failed to convert a Polygon");
                // bump the Polygon counter
                ctr.store(ctr.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                // generate a label position Point for it, and put it back
                Value::from(&polylabel(&geo_type, tolerance))
            }
            Value::MultiPolygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let geo_type: MultiPolygon<f32> = intermediate
                    .try_into()
                    .expect("Failed to convert a MultiPolygon");
                // we allocate here – can we avoid it? idk
                let mp = MultiPoint(
                    geo_type
                        .0
                        .par_iter()
                        .map(|polygon| {
                            // bump the Polygon counter
                            ctr.store(ctr.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                            // generate a label position
                            polylabel(polygon, tolerance)
                        })
                        .collect(),
                );
                // move label positions into geometry
                Value::from(&mp)
            }
            _ => replace(&mut gmt.value, Value::from(&fake_polygon)),
        }
    }
}

/// Convert any `GeoJson` enum variant into a `GeoJson::FeatureCollection`
fn build_featurecollection(gj: GeoJson) -> GeoJson {
    match gj {
        GeoJson::FeatureCollection(fc) => GeoJson::FeatureCollection(fc),
        GeoJson::Feature(f) => GeoJson::FeatureCollection(FeatureCollection {
            bbox: None,
            features: vec![f],
            foreign_members: None,
        }),
        GeoJson::Geometry(g) => GeoJson::FeatureCollection(FeatureCollection {
            bbox: None,
            features: vec![
                Feature {
                    bbox: None,
                    geometry: Some(g),
                    id: None,
                    properties: Some(Map::new()),
                    foreign_members: None,
                },
            ],
            foreign_members: None,
        }),
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
            let ctr = AtomicIsize::new(0);
            process_geojson(&mut gj, &tolerance, &ctr);
            // Always return a FeatureCollection
            // This can allocate, but there's no way around that
            gj = build_featurecollection(gj);
            let to_print = if !pprint {
                gj.to_string()
            } else {
                to_string_pretty(&gj).unwrap()
            };
            if user_attended() {
                println!(
                    "Processing complete. Labelled {} Polygons\n",
                    style(&ctr.load(Ordering::Relaxed).to_string()).red()
                );
        }
            println!("{}", to_print);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let mut gj = open_and_parse(&"geojson/geometrycollection_nested.geojson").unwrap();
        let ctr = AtomicIsize::new(0);
        process_geojson(&mut gj, &0.001, &ctr);
        gj = build_featurecollection(gj);
        assert_eq!(gj, correct);
    }
}
