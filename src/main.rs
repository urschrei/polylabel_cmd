use std::fs::File;
// use std::io::{Write, BufWriter};
use std::io::prelude::*;
use std::process;
use std::error::Error;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
use geo::{MultiPolygon, MultiPoint, Point};

extern crate geojson;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate serde_json;
use serde_json::{Map, Value as Sdv};

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

/// build a Point feature
fn build_feature(
    geom: &Point<f32>,
    id: Option<Sdv>,
    properties: Option<Map<String, Sdv>>,
    fm: Option<Map<String, Sdv>>,
) -> Feature {
    Feature {
        bbox: None,
        geometry: Some(Geometry::new(Value::from(geom))),
        id: id,
        properties: properties,
        foreign_members: fm,
    }
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
        let results: Option<_> = match gj {
            GeoJson::FeatureCollection(fc) => {
                let processed: Vec<_> = fc.features
                    .into_par_iter()
                    .filter_map(|feature| {
                        match feature.geometry {
                            Some(geometry) => match geometry.value {
                                Value::Polygon(_) => {
                                    let res =
                                        polylabel(&geometry.value.try_into().unwrap(), &tolerance);
                                    Some(vec![
                                        build_feature(
                                            &res,
                                            feature.id,
                                            feature.properties,
                                            feature.foreign_members,
                                        ),
                                    ])
                                }
                                // This will discard the MultiPolygon properties
                                // How to iterate over the Polygons in a GeoJson MultiPolygon?
                                Value::MultiPolygon(_) => {
                                    // TODO: MultiPolygon should map to MultiPoint
                                    let mp: MultiPolygon<_> = geometry
                                        .value
                                        .try_into()
                                        .expect("Unable to convert MultiPolygon");
                                    let results: Vec<Point<_>> = mp.0
                                        .par_iter()
                                        .map(|poly| polylabel(poly, &tolerance))
                                        .collect();
                                    Some(
                                        results
                                            .into_par_iter()
                                            .map(|point| build_feature(&point, None, None, None))
                                            .collect::<Vec<Feature>>(),
                                    )
                                }
                                // only Polygons are allowed
                                _ => None,
                            },
                            // empty feature
                            _ => None,
                        }
                    })
                    .flat_map(|f| f)
                    .collect();
                Some(FeatureCollection {
                    bbox: fc.bbox,
                    features: processed,
                    foreign_members: fc.foreign_members,
                })
            }
            // A single feature
            GeoJson::Feature(feature) => {
                match feature.geometry {
                    Some(geometry) => match geometry.value {
                        // A single polygon
                        Value::Polygon(_) => {
                            let res = polylabel(&geometry.value.try_into().unwrap(), &tolerance);
                            Some(FeatureCollection {
                                bbox: None,
                                features: vec![build_feature(&res, None, None, None)],
                                foreign_members: None,
                            })
                        }
                        // This will discard the MultiPolygon properties
                        // How to iterate over the Polygons in a GeoJson MultiPolygon?
                        Value::MultiPolygon(_) => {
                            // TODO: MultiPolygon should map to MultiPoint
                            let mp: MultiPolygon<_> = geometry
                                .value
                                .try_into()
                                .expect("Unable to convert MultiPolygon");
                            let results: Vec<Point<_>> = mp.0
                                .iter()
                                .map(|poly| polylabel(poly, &tolerance))
                                .collect();
                            Some(FeatureCollection {
                                bbox: feature.bbox,
                                features: results
                                    .into_par_iter()
                                    .map(|point| build_feature(&point, None, None, None))
                                    .collect::<Vec<Feature>>(),
                                foreign_members: feature.foreign_members,
                            })
                        }
                        // only Polygons are allowed
                        _ => None,
                    },
                    // empty feature
                    _ => None,
                }
            }
            GeoJson::Geometry(geometry) => {
                match geometry.value {
                    Value::Polygon(_) => {
                        let res = polylabel(&geometry.value.try_into().unwrap(), &tolerance);
                        Some(FeatureCollection {
                            bbox: None,
                            features: vec![build_feature(&res, None, None, None)],
                            foreign_members: None,
                        })
                    }
                    Value::MultiPolygon(_) => {
                        // TODO: MultiPolygon should map to MultiPoint
                        let mp: MultiPolygon<_> = geometry
                            .value
                            .try_into()
                            .expect("Unable to convert MultiPolygon");
                        let results: Vec<Point<_>> = mp.0
                            .iter()
                            .map(|poly| polylabel(poly, &tolerance))
                            .collect();
                        // let mpoint = MultiPoint(results);
                        Some(FeatureCollection {
                            bbox: None,
                            features: results
                                .into_par_iter()
                                .map(|point| build_feature(&point, None, None, None))
                                .collect::<Vec<Feature>>(),
                            foreign_members: None,
                        })
                    }
                    // only Polygons are allowed
                    _ => None,
                }
            }
        };
        if results.is_some() {
            let f = results.unwrap();
            let serialised = GeoJson::from(f).to_string();
            println!("{}", serialised);
        } else {
            println!("No valid polygons were found. Please check your input.");
        }
    }
}
