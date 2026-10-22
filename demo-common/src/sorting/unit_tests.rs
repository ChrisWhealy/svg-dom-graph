use super::array::{HIGHEST, LEN, LOWEST, sample};
use super::bars::{bar_height, bar_size};

#[test]
fn a_sample_is_len_different_elements_in_range() {
    let mut seed = 0.37_f64;
    let mut random = move || {
        seed = (seed * 9301.0 + 49297.0) % 233_280.0;
        seed / 233_280.0
    };
    for _ in 0..50 {
        let picked = sample(&mut random);
        assert_eq!(picked.len(), LEN);
        assert!(picked.iter().all(|&v| (LOWEST..=HIGHEST).contains(&v)));
        let mut unique = picked.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), LEN, "{picked:?}");
    }
}

#[test]
fn a_sample_copes_with_the_largest_number_the_source_can_give() {
    assert_eq!(sample(|| 0.999_999_999).len(), LEN);
    assert_eq!(sample(|| 0.0).len(), LEN);
}

#[test]
fn the_height_of_a_bar_is_in_proportion_to_its_value() {
    assert!((bar_height(20) - 2.0 * bar_height(10)).abs() < 1e-9);
    assert!(bar_height(HIGHEST) > bar_height(LOWEST));
    assert!(bar_height(LOWEST) > 0.0);
}

#[test]
fn every_bar_is_thin_and_as_tall_as_its_value() {
    for value in LOWEST..=HIGHEST {
        let size = bar_size(value);
        assert!(size.width < size.height + 1.0 || size.width < 60.0);
        assert!((size.height - bar_height(value)).abs() < 1e-9);
    }
}
