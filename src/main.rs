use std::fs::File;
// use std::io::{Write, BufWriter};
use std::io::prelude::*;
use std::process;
use std::error::Error;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
use geo::{MultiPolygon};

extern crate geojson;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate serde_json;
use serde_json::Map;

extern crate polylabel;
use polylabel::polylabel;

extern crate rayon;
use rayon::prelude::*;

fn open_and_parse(p: &str) -> Result<GeoJson, Box<Error>> {
    let mut f = File::open(p)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents.parse::<GeoJson>()?)
}

fn main() {
    let command_params = App::new("polylabel")
       .version(&crate_version!()[..])
       .author("Stephan Hügel <urschrei@gmail.com>")
       .about("Find optimum label positions for polygons")
       .args_from_usage("-t --tolerance=[TOLERANCE] 'Set a tolerance for finding the label position. Defaults to 0.001'")
       .arg(Arg::with_name("GEOJSON")
                .help("GeoJSON with a FeatureCollection containing one or more (multi)polygons, \
                 or a Feature containing a multi(polygon) or a geometry that is a (multi)polygon")
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
        // This will hold Point<_> values
        let mut results: Vec<Option<_>> = match gj {
            GeoJson::FeatureCollection(fc) => fc.features
                .into_par_iter()
                // filter_map removes any None values
                .filter_map(|feature| match feature.geometry {
                    Some(geometry) => match geometry.value {
                        Value::Polygon(_) => {
                            Some(vec![polylabel(&geometry.value.try_into().expect(
                                "Unable to convert Polygon"), &tolerance)])
                        },
                        Value::MultiPolygon(_) => {
                            let mp: MultiPolygon<_> = geometry.value.try_into().expect(
                                "Unable to convert MultiPolygon");
                            Some(mp.0.iter().map(|poly| polylabel(poly, &tolerance)).collect())
                        },
                        // only Polygons are allowed
                        _ => None,
                    },
                    // empty feature
                    _ => None,
                })
                .map(|p| Some(p))
                .collect(),
            GeoJson::Feature(feature) => {
                match feature.geometry {
                    Some(geometry) => match geometry.value {
                        Value::Polygon(_) => {
                            vec![Some(vec![polylabel(&geometry.value.try_into().expect(
                                "Unable to convert Polygon"), &tolerance)])]
                        },
                        Value::MultiPolygon(_) => {
                            let mp: MultiPolygon<_> = geometry.value.try_into().expect(
                                "Unable to convert MultiPolygon");
                            vec![Some(mp.0.iter().map(|poly| polylabel(poly, &tolerance)).collect())]
                        },
                        // only Polygons are allowed
                        _ => vec![None],
                    },
                    // empty feature
                    _ => vec![None]
                }
            },
            GeoJson::Geometry(geometry) => {
                match geometry.value {
                    Value::Polygon(_) => {
                        vec![Some(vec![polylabel(&geometry.value.try_into().expect(
                            "Unable to convert Polygon"), &tolerance)])]
                    },
                    Value::MultiPolygon(_) => {
                        let mp: MultiPolygon<_> = geometry.value.try_into().expect(
                            "Unable to convert MultiPolygon");
                        vec![Some(mp.0.iter().map(|poly| polylabel(poly, &tolerance)).collect())]
                    },
                    // only Polygons are allowed
                    _ => vec![None],
                }
            }
        };
        results.retain(|vec| vec.is_some());
        if !results.is_empty() {
            // Build an output geojson
            // results is a Vec<Option<Vec<Point<_>>>>
            // flat_map removes the inner vec, yielding Option<Point<_>>
            let feature_collection = FeatureCollection {
                bbox: None,
                features: results
                    .into_par_iter()
                    .flat_map(|points| points.unwrap())
                    .map(|point| Value::from(&point))
                    .map(|value| Feature {
                        bbox: None,
                        geometry: Some(Geometry::new(value)),
                        id: None,
                        properties: Some(Map::new()),
                        foreign_members: None,
                    })
                    .collect(),
                foreign_members: None,
            };
            let serialised = GeoJson::from(feature_collection).to_string();
            println!("{}", serialised);
        } else {
            println!("No valid polygons were found. Please check your input.");
        }
    }
}
