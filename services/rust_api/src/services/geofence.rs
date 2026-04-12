use geo::algorithm::contains::Contains;
use geo::{LineString, Point, Polygon};

/// Create a sample square polygon (10x10)
#[allow(dead_code)]
pub fn sample_polygon() -> Polygon {
    Polygon::new(
        LineString::from(vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ]),
        vec![], // No interior holes
    )
}

/// Returns true if the point is inside the polygon
#[allow(dead_code)]
pub fn is_point_inside(p: Point, poly: &Polygon) -> bool {
    poly.contains(&p)
}
