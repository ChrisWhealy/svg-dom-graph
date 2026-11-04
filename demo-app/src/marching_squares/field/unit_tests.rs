use super::*;

#[test]
fn every_field_has_a_grid_and_ascending_levels() {
    for field in FieldId::all() {
        let grid = field.grid().expect("a preset is valid");
        assert_eq!((grid.width, grid.height), field.size(), "{field:?}");
        let levels = field.levels();
        assert!(levels.windows(2).all(|w| w[0] < w[1]), "{field:?}");
    }
}

#[test]
fn the_keys_are_unique_and_found_again() {
    let all = FieldId::all();
    assert_eq!(all.len(), 8);
    for field in &all {
        assert_eq!(FieldId::from_key(field.key()), Some(*field));
        assert!(!field.label().is_empty());
    }
    let mut keys: Vec<&str> = all.iter().map(|f| f.key()).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), all.len());
    assert_eq!(FieldId::from_key("nothing"), None);
}

#[test]
fn the_grids_are_sampled_as_the_spec_says() {
    let ripple = FieldId::Ripple.grid().expect("grid");
    // Row 0 is the top, y = 2.25, and column 0 is x = -3.
    let (x, y) = (-3.0_f64, 2.25_f64);
    assert_eq!(ripple.at(0, 0), (1.15 * x).sin() * (1.35 * y).cos() + 0.16 * x - 0.08 * y);
    let last = ripple.at(18, 24);
    let (x, y) = (3.0_f64, -2.25_f64);
    assert!((last - ((1.15 * x).sin() * (1.35 * y).cos() + 0.16 * x - 0.08 * y)).abs() < 1e-12);
    let coarse = FieldId::Coarse.grid().expect("grid");
    assert_eq!(coarse.at(0, 0), (-3.5_f64).sin() * (2.5_f64).sin());
    assert_eq!(coarse.at(6, 8), (4.5_f64).sin() * (-3.5_f64).sin());
    let lab = FieldId::Lab(LabCase::FivePositive).grid().expect("grid");
    assert_eq!((lab.at(0, 0), lab.at(0, 1), lab.at(1, 1), lab.at(1, 0)), (3.0, -1.0, 1.0, -2.0));
}

#[test]
fn only_fields_of_one_size_share_a_view() {
    assert!(FieldId::Ripple.same_extent(FieldId::Twin));
    assert!(!FieldId::Ripple.same_extent(FieldId::Coarse));
    assert!(!FieldId::Coarse.same_extent(FieldId::Lab(LabCase::Tie)));
    assert!(FieldId::Lab(LabCase::Tie).same_extent(FieldId::Lab(LabCase::FivePositive)));
}

#[test]
fn the_three_slots_differ_by_pattern_as_well_as_colour() {
    assert_eq!(SLOT_DASHES[0], None);
    assert!(SLOT_DASHES[1].is_some() && SLOT_DASHES[2].is_some() && SLOT_DASHES[1] != SLOT_DASHES[2]);
    let mut colours = SLOT_COLOURS.to_vec();
    colours.dedup();
    assert_eq!(colours.len(), LEVELS);
    assert_eq!(SLOT_PATTERNS.len(), LEVELS);
}
