use std::fs::File;
// use std::io::{Write, BufWriter};
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

type ValMap = Option<Map<String, Sdv>>;

/// build a Feature from an input Geo type
fn build_feature<'a, G>(geom: &'a G, id: Option<Sdv>, prp: ValMap, fm: ValMap) -> Feature
where
    Value: From<&'a G>,
    G: 'a,
{
    Feature {
        bbox: None,
        geometry: Some(Geometry::new(Value::from(geom))),
        id: id,
        properties: prp,
        foreign_members: fm,
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
                let processed: Vec<_> = collection.features
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
                                // How to iterate over the Polygons in a GeoJson MultiPolygon?
                                Value::MultiPolygon(_) => {
                                    // MultiPolygons map to MultiPoints
                                    let mp: MultiPolygon<_> = geometry
                                        .value
                                        .try_into()
                                        .expect("Unable to convert MultiPolygon");
                                    let results = MultiPoint(
                                        mp.0
                                            .par_iter()
                                            .map(|poly| polylabel(poly, &tolerance))
                                            .collect(),
                                    );
                                    Some(vec![
                                        build_feature(&results, None, feature.properties, None),
                                    ])
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
                // FINALLY, build a FeatureCollection out of this insanity
                Some(FeatureCollection {
                    bbox: collection.bbox,
                    features: processed,
                    foreign_members: collection.foreign_members,
                })
            },
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
                        // How to iterate over the Polygons in a GeoJson MultiPolygon?
                        Value::MultiPolygon(_) => {
                            // MultiPolygons map to MultiPoints
                            let mp: MultiPolygon<_> = geometry
                                .value
                                .try_into()
                                .expect("Unable to convert MultiPolygon");
                            let results = MultiPoint(
                                mp.0
                                    .par_iter()
                                    .map(|poly| polylabel(poly, &tolerance))
                                    .collect(),
                            );
                            Some(FeatureCollection {
                                bbox: None,
                                features: vec![
                                    build_feature(&results, None, feature.properties, None),
                                ],
                                foreign_members: None,
                            })
                        }
                        // only Polygons are allowed
                        _ => None,
                    },
                    // empty feature
                    _ => None,
                }
            },
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
                        // MultiPolygons map to MultiPoints
                        let mp: MultiPolygon<_> = geometry
                            .value
                            .try_into()
                            .expect("Unable to convert MultiPolygon");
                        let results = MultiPoint(
                            mp.0
                                .par_iter()
                                .map(|poly| polylabel(poly, &tolerance))
                                .collect(),
                        );
                        Some(FeatureCollection {
                            bbox: None,
                            features: vec![build_feature(&results, None, None, None)],
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
