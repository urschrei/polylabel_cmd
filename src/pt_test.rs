    extern crate geo;
    use geo::Polygon;

    extern crate geojson;
    use geojson::conversion::TryInto;
    use geojson::Value;


    fn main() {

        let coord1 = vec![100.0, 0.0];
        let coord2 = vec![101.0, 1.0];
        let coord3 = vec![101.0, 1.0];
        let coord4 = vec![104.0, 0.2];
        let coord5 = vec![100.9, 0.2];
        let coord6 = vec![100.9, 0.7];

        let geojson_multi_line_string_type1 =
            vec![vec![coord1.clone(), coord2.clone(), coord3.clone(), coord1.clone()],
                 vec![coord4.clone(), coord5.clone(), coord6.clone(), coord4.clone()]];

        let geojson_polygon = Value::Polygon(geojson_multi_line_string_type1);
        let gj2 = geojson_polygon.clone();
        let p: geo::Polygon<f64> = geojson_polygon.try_into().unwrap();
        println!("Exterior shell: {:?}", p.exterior);

        // this doesn't compile
        let mut fc = vec![];
        fc.push(gj2);
        let geometries: Vec<Polygon<f64>> = fc.into_iter()
            .map(|feature| match feature {
                     Value::Polygon(pt) => pt.try_into().unwrap(),
                     _ => Polygon::new(vec![].into(), vec![]),
                 })
            .collect();
    }
