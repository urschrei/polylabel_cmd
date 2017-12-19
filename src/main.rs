use std::fs::File;
use std::io::prelude::*;
use std::process;
use std::error::Error;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
use geo::{MultiPoint, MultiPolygon};

extern crate geojson;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate serde_json;
use serde_json::Map;

extern crate polylabel;
use polylabel::polylabel;

extern crate rayon;
use rayon::prelude::*;

/// Attempt to open a file, read it, and parse it into GeoJSON
fn open_and_parse(p: &str) -> Result<GeoJson, Box<Error>> {
    let mut f = File::open(p)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents.parse::<GeoJson>()?)
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
/// GeometryCollections are processed recursively, so
/// [nested](https://tools.ietf.org/html/rfc7946#section-3.1.8) collections
/// are successfully processed, but please don't do that.
fn label_for_geometry(geom: Geometry, tolerance: &f32) -> Option<Geometry> {
    match geom.value {
        Value::Polygon(_) => Some(Geometry::new(Value::from(&polylabel(
            &geom.value.try_into().expect("Unable to convert Polygon"),
            tolerance,
        )))),
        // How to iterate over the Polygons in a GeoJson MultiPolygon?
        Value::MultiPolygon(_) => {
            // MultiPolygons map to MultiPoints
            let mp: MultiPolygon<_> = geom.value
                .try_into()
                .expect("Unable to convert MultiPolygon");
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
                        .map(|geom_| {
                            // Recur!
                            label_for_geometry(geom_, tolerance)
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
       .arg(Arg::with_name("GEOJSON")
                .help("GeoJSON with a FeatureCollection containing one or more (Multi)Polygons, \
                 or a Feature containing a Multi(Polygon), or a Geometry that is a (Multi)Polygon")
                .index(1)
                .required(true))
       .get_matches();

    let tolerance = value_t!(command_params.value_of("TOLERANCE"), f32).unwrap_or(0.001);
    let poly = value_t!(command_params.value_of("GEOJSON"), String).unwrap();
    let res = open_and_parse(&poly);
    if res.is_err() {
        println!("An error occurred: {:?}", res.err().unwrap());
        process::exit(1);
    } else {
        let gj = res.unwrap();
        let results: Option<_> = match gj {
            GeoJson::FeatureCollection(collection) => {
                let processed: Vec<_> = collection
                    .features
                    .into_par_iter()
                    // filter_map will remove any None features
                    .filter_map(|feature| label_for_feature(feature, &tolerance))
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
            GeoJson::Feature(feature) => match label_for_feature(feature, &tolerance) {
                Some(labelled_feature) => Some(FeatureCollection {
                    bbox: None,
                    features: vec![labelled_feature],
                    foreign_members: None,
                }),
                None => None,
            },
            GeoJson::Geometry(geometry) => match label_for_geometry(geometry, &tolerance) {
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
        };
        if results.is_some() {
            let f = results.unwrap();
            let serialised = GeoJson::from(f).to_string();
            println!("{}", serialised);
        } else {
            println!("No valid geometries were found. Please check your input.");
        }
    }
}
