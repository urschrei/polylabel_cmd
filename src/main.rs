use anyhow::{Context, Result};
use clap::{crate_version, value_t, App, Arg};
use console::{style, user_attended};
use geo_types::{LineString, MultiPoint, MultiPolygon, Point, Polygon};
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value};
use indicatif::ProgressBar;
use polylabel::polylabel;
use rayon::prelude::*;
use serde_json::{to_string_pretty, Map};
use std::mem::replace;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::{convert::TryInto, fs};

/// Attempt to open a file, read it, and parse it into `GeoJSON`
fn open_and_parse<P>(filename: P) -> Result<GeoJson>
where
    P: AsRef<Path>,
{
    let s = fs::read_to_string(filename)
        .with_context(|| "Couldn't open or read from the file. Check that it exists?")?;
    Ok(s.parse::<GeoJson>()
        .with_context(|| "Couldn't parse GeoJSON from the file. Check that it's valid?")?)
}

/// Process top-level `GeoJSON` items
fn process_geojson(gj: &mut GeoJson, tolerance: f64, ctr: &AtomicIsize) {
    match *gj {
        GeoJson::FeatureCollection(ref mut collection) => collection
            .features
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
fn label_geometry(geom: &mut Geometry, tolerance: f64, ctr: &AtomicIsize) {
    match geom.value {
        Value::Polygon(_) | Value::MultiPolygon(_) => label_value(Some(geom), tolerance, ctr),
        Value::GeometryCollection(ref mut collection) => {
            // GeometryCollections contain other Geometry types, and can nest
            // we deal with this by recursively processing each geometry
            collection
                .par_iter_mut()
                .for_each(|geometry| label_geometry(geometry, tolerance, ctr))
        }
        // Any other geometry: leave unchanged
        _ => {}
    }
}

/// Generate a label position for a (Multi)Polygon Value
fn label_value(geom: Option<&mut Geometry>, tolerance: f64, ctr: &AtomicIsize) {
    if let Some(gmt) = geom {
        // construct a fake empty Polygon – this doesn't allocate
        // TODO if Geo geometry validation lands, this will fail
        let v1: Vec<Point<f64>> = Vec::new();
        let ls2 = Vec::new();
        let fake_polygon: Polygon<f64> = Polygon::new(LineString::from(v1), ls2);
        // convert it into a Value, and swap it for our actual (Multi)Polygon
        gmt.value = match gmt.value {
            Value::Polygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let geo_type: Polygon<f64> = intermediate
                    .try_into()
                    .expect("Failed to convert a Polygon");
                // bump the Polygon counter
                ctr.fetch_add(1, Ordering::SeqCst);
                // generate a label position Point for it, and put it back
                Value::from(
                    &polylabel(&geo_type, &tolerance)
                        .expect("Couldn't build a label Point for the input polygon"),
                )
            }
            Value::MultiPolygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let geo_type: MultiPolygon<f64> = intermediate
                    .try_into()
                    .expect("Failed to convert a MultiPolygon");
                // we allocate here – can we avoid it? idk
                let mp = MultiPoint(
                    geo_type
                        .iter()
                        .map(|polygon| {
                            // bump the Polygon counter
                            ctr.fetch_add(1, Ordering::SeqCst);
                            // generate a label position
                            polylabel(polygon, &tolerance)
                                .expect("Couldn't build a label Point for the input MultiPolygon")
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
            features: vec![Feature {
                bbox: None,
                geometry: Some(g),
                id: None,
                properties: Some(Map::new()),
                foreign_members: None,
            }],
            foreign_members: None,
        }),
    }
}

fn main() -> Result<()> {
    let command_params = App::new("polylabel")
        .version(&crate_version!()[..])
        .author("Stephan Hügel <urschrei@gmail.com>")
        .about("Find optimum label positions for polygons")
        .arg(
            Arg::with_name("tolerance")
                .takes_value(true)
                .help(
                    "Set a tolerance for finding \
        the label position. Defaults to 0.001",
                )
                .short("t")
                .long("tolerance"),
        )
        .arg(
            Arg::with_name("pretty")
                .help("Pretty-print GeoJSON output")
                .short("p")
                .long("pretty"),
        )
        .arg(
            Arg::with_name("statsonly")
                .help("Label polygons, but only print stats")
                .short("s")
                .long("stats-only"),
        )
        .arg(
            Arg::with_name("GEOJSON")
                .help(
                    "GeoJSON with a FeatureCollection containing one or more (Multi)Polygons, \
                 or a Feature containing a Multi(Polygon), or a Geometry that is a (Multi)Polygon, \
                 or a GeometryCollection containing (Multi)Polygons.",
                )
                .index(1)
                .required(true),
        )
        .get_matches();

    let tolerance = value_t!(command_params.value_of("tolerance"), f64).unwrap_or(0.001);
    let poly = value_t!(command_params.value_of("GEOJSON"), String).unwrap();
    let pprint = command_params.is_present("pretty");
    let statsonly = command_params.is_present("statsonly");
    let sp = ProgressBar::new_spinner();
    sp.set_message("Parsing GeoJSON");
    sp.enable_steady_tick(1);
    let res = open_and_parse(&poly);
    sp.finish_and_clear();
    let sp2 = ProgressBar::new_spinner();
    sp2.set_message("Labelling…");
    sp2.enable_steady_tick(1);
    match res {
        Err(e) => Err(e),
        Ok(mut gj) => {
            let ctr = AtomicIsize::new(0);
            process_geojson(&mut gj, tolerance, &ctr);
            // Always return a FeatureCollection
            // This can allocate, but there's no way around that
            if !statsonly {
                gj = build_featurecollection(gj);
            }
            sp2.finish_and_clear();
            let to_print = if !pprint {
                gj.to_string()
            } else {
                to_string_pretty(&gj).unwrap()
            };
            if user_attended() {
                let labelled = ctr.load(Ordering::Relaxed);
                println!(
                    "Processing complete. Labelled {} {}\n",
                    style(&labelled.to_string()).red(),
                    if labelled == 1 { "Polygon" } else { "Polygons" }
                );
            }
            if !statsonly {
                println!("{}", to_print);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::GeoJson;
    #[test]
    // Can a nested GeometryCollection be parsed?
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
        process_geojson(&mut gj, 0.001, &ctr);
        gj = build_featurecollection(gj);
        assert_eq!(gj, correct);
    }
    #[test]
    // London geometry
    fn test_london() {
        let raw_gj = r#"
            {
            "type": "FeatureCollection",
            "features": [
            {
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Point",
                    "coordinates": [-0.455568164459203567, 51.54848888202886]
                }
            }
            ]
            }
        "#;
        let correct = raw_gj.parse::<GeoJson>().unwrap();
        let mut gj = open_and_parse(&"geojson/london_polygon.geojson").unwrap();
        let ctr = AtomicIsize::new(0);
        process_geojson(&mut gj, 0.001, &ctr);
        assert_eq!(gj, correct);
    }
}
