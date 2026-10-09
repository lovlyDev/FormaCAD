use super::*;
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<f64> {
    vec![x, y, x + w, y, x + w, y + h, x, y + h]
}
#[test]
fn holes_are_classified_independently_of_order_and_winding() {
    let outer = rectangle(0.0, 0.0, 40.0, 30.0);
    let hole = rectangle(5.0, 5.0, 5.0, 5.0);
    let other = rectangle(20.0, 10.0, 4.0, 8.0);
    let (boundary, holes) = classify(vec![hole, outer.clone(), other], "profile").unwrap();
    assert_eq!(boundary, outer);
    assert_eq!(holes.len(), 2);
    assert!(holes.iter().all(|h| area(h) < 0.0));
}
#[test]
fn touching_crossing_external_and_nested_holes_are_rejected() {
    let outer = rectangle(0.0, 0.0, 40.0, 30.0);
    for hole in [
        rectangle(0.0, 5.0, 5.0, 5.0),
        rectangle(38.0, 5.0, 5.0, 5.0),
        rectangle(50.0, 5.0, 5.0, 5.0),
    ] {
        assert!(classify(vec![outer.clone(), hole], "profile").is_err());
    }
    assert!(classify(
        vec![
            outer.clone(),
            rectangle(5.0, 5.0, 20.0, 20.0),
            rectangle(10.0, 10.0, 5.0, 5.0)
        ],
        "profile"
    )
    .is_err());
    assert!(classify(
        vec![
            outer,
            rectangle(5.0, 5.0, 10.0, 10.0),
            rectangle(10.0, 10.0, 10.0, 10.0)
        ],
        "profile"
    )
    .is_err());
}
