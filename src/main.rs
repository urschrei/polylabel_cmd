use std::fs::File;
// use std::io::{Write, BufWriter};
use std::io::prelude::*;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo;
// use geo::{Polygon, Point};

extern crate geojson;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate serde_json;
use serde_json::Map;

extern crate polylabel;
use polylabel::polylabel;

extern crate rayon;
use rayon::prelude::*;

fn main() {
    let command_params = App::new("polylabel")
       .version(&crate_version!()[..])
       .author("Stephan Hügel <urschrei@gmail.com>")
       .about("Find optimum label positions for polygons")
       .args_from_usage("-t --tolerance=[TOLERANCE] 'Set a tolerance for finding the label position. Defaults to 1.0'")
       .arg(Arg::with_name("GEOJSON")
                .help("GeoJSON with a FeatureCollection containing one or more polygons")
                .index(1)
                .required(true))
       .get_matches();

    let tolerance = value_t!(command_params.value_of("TOLERANCE"), f32).unwrap_or(1.0);
    let poly = value_t!(command_params.value_of("GEOJSON"), String).unwrap();
    let mut f = File::open(poly).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("Unable to read file");
    let gj = contents.parse::<GeoJson>().unwrap();
    // This will hold Point<_> values
    let results: Vec<Option<_>> = match gj {
        GeoJson::FeatureCollection(fc) => fc.features
            .into_par_iter()
            // filter_map removes any None values from the final collection
            .filter_map(|feature| match feature.geometry {
                Some(geometry) => match geometry.value {
                    Value::Polygon(_) => {
                        Some(polylabel(&geometry.value.try_into().unwrap(), &tolerance))
                    }
                    Value::Point(_) => None,
                    // only Polygons are allowed
                    _ => None,
                },
                // only valid Features are allowed
                _ => None,
            })
            .map(|p| Some(p))
            .collect(),
        // only FeatureCollections are allowed
        _ => vec![None],
    };
    // now build an output geojson
    let feature_collection = FeatureCollection {
        bbox: None,
        features: results
            .into_par_iter()
            .map(|point| Value::from(&point.unwrap()))
            .map(|value| {
                Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(value)),
                    id: None,
                    properties: Some(Map::new()),
                    foreign_members: None,
                }
            })
            .collect(),
        foreign_members: None,
    };
    let serialised = GeoJson::from(feature_collection).to_string();
    println!("{}", serialised);
}
