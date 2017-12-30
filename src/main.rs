use std::fs::File;
use std::io::prelude::*;
use std::io::Error as IoErr;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
use geo::{MultiPoint, MultiPolygon};

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
    #[fail(display = "IO error: {}", _0)]
    IoError(#[cause] IoErr),
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

/// Generate a FeatureCollection of label positions from an input GeoJson enum
fn label_for_geojson(gj: GeoJson, tolerance: &f32) -> Option<FeatureCollection> {
    match gj {
        GeoJson::FeatureCollection(collection) => {
            let processed: Vec<_> = collection
                .features
                .into_par_iter()
                // filter_map will remove any None features
                .filter_map(|feature| label_for_feature(feature, tolerance))
                .collect();
            if processed.is_empty() {
                None
            } else {
                Some(FeatureCollection {
                    bbox: collection.bbox,
                    features: processed,
                    foreign_members: collection.foreign_members,
                })
            }
        }
        GeoJson::Feature(feature) => match label_for_feature(feature, tolerance) {
            Some(labelled_feature) => Some(FeatureCollection {
                bbox: None,
                features: vec![labelled_feature],
                foreign_members: None,
            }),
            None => None,
        },
        GeoJson::Geometry(geometry) => match label_for_geometry(geometry, tolerance) {
            Some(labelled_geometry) => {
                let f = Feature {
                    bbox: None,
                    geometry: Some(labelled_geometry),
                    id: None,
                    properties: Some(Map::new()),
                    foreign_members: None,
                };
                Some(FeatureCollection {
                    bbox: None,
                    features: vec![f],
                    foreign_members: None,
                })
            }
            None => None,
        },
    }
}

/// Generate a Feature containing label positions
fn label_for_feature(feat: Feature, tolerance: &f32) -> Option<Feature> {
    match feat.geometry {
        Some(geom) => match label_for_geometry(geom, tolerance) {
            Some(ngeom) => Some(Feature {
                bbox: feat.bbox,
                geometry: Some(ngeom),
                id: feat.id,
                properties: feat.properties,
                foreign_members: feat.foreign_members,
            }),
            _ => None,
        },
        None => None,
    }
}

/// Generate a Geometry containing label positions from an input geometry
/// Input and output geometries are symmetrical.
/// `GeometryCollection`s are processed recursively, so
/// [nested](https://tools.ietf.org/html/rfc7946#section-3.1.8) collections
/// are successfully processed, but please don't do that.
fn label_for_geometry(geom: Geometry, tolerance: &f32) -> Option<Geometry> {
    match geom.value {
        Value::Polygon(_) => Some(Geometry::new(Value::from(&polylabel(
            &geom.value.try_into().ok()?,
            tolerance,
        )))),
        // How to iterate over the Polygons in a GeoJson MultiPolygon?
        Value::MultiPolygon(_) => {
            // MultiPolygons map to MultiPoints
            let mp: MultiPolygon<_> = geom.value.try_into().ok()?;
            Some(Geometry::new(Value::from(&MultiPoint(
                mp.0
                    .par_iter()
                    .map(|poly| polylabel(poly, tolerance))
                    .collect(),
            ))))
        }
        Value::GeometryCollection(gc) => {
            Some(Geometry {
                bbox: None,
                value: Value::GeometryCollection(
                    gc.into_par_iter()
                        .map(|collectionitem| {
                            // Recur!
                            label_for_geometry(collectionitem, tolerance)
                        })
                        .filter_map(|f| f)
                        .collect(),
                ),
                foreign_members: None,
            })
        }
        // only (Multi)Polygons or GeometryCollections are allowed
        _ => None,
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
        Ok(gj) => {
            let results: Option<_> = label_for_geojson(gj, &tolerance);
            if results.is_some() {
                let f = results.unwrap();
                let serialised = GeoJson::from(f);
                let to_print = if !pprint {
                    serialised.to_string()
                } else {
                    to_string_pretty(&serialised).unwrap()
                };
                println!("{}", to_print);
            } else {
                println!("No valid geometries were found. Please check your input.");
            }
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
