use std::fs::File;
use std::io::prelude::*;

#[macro_use]
extern crate clap;
use clap::{Arg, App};

extern crate geo;
use geo::{Polygon};

extern crate geojson;
use geojson::{GeoJson, Geometry, Value};
use geojson::conversion::TryInto;

extern crate polylabel;
use polylabel::polylabel;

fn main() {
    let command_params = App::new("polylabel")
       .version(&crate_version!()[..])
       .author("Stephan Hügel <urschrei@gmail.com>")
       .about("Find optimum label positions for polygons")
       .args_from_usage("-t --tolerance=[TOLERANCE] 'Set a tolerance for finding the label position. Defaults to 1.0'")
       .arg(Arg::with_name("GEOJSON")
                .help("A GeoJSON file representing a polygon")
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
    match gj {
        GeoJson::FeatureCollection(fc) => {
            let geometries: Vec<Polygon<f64>> = fc.features
                .into_iter()
                .filter_map(|feature| match feature.geometry {
                                Some(geometry) => {
                                    match geometry.value {
                                        Value::Polygon(_) => geometry.value.try_into().ok(),
                                        Value::Point(_) => None,
                                        _ => None,
                                    }
                                }
                                _ => None,
                            })
                .collect();
            println!("{:?}", geometries);
        }
        GeoJson::Feature(f) => {
            println!("{:?}", f.bbox);
            println!("{:?}", f.bbox);
        }
        GeoJson::Geometry(g) => {
            println!("{:?}", g.value);
            println!("{:?}", g.value);
        }
    }

}
