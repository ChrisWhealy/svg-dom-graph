use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn numbers_come_out_of_a_transform_and_a_path_in_order() -> Result<(), String> {
    let cases: [(&str, &[f64]); 6] = [
        ("translate(20, 20)", &[20.0, 20.0]),
        ("translate(-5.5, 7)", &[-5.5, 7.0]),
        ("M 100 40 L 200 40 L 200 170", &[100.0, 40.0, 200.0, 40.0, 200.0, 170.0]),
        ("M100,40L200,40", &[100.0, 40.0, 200.0, 40.0]),
        ("M 76 36 L 204 100", &[76.0, 36.0, 204.0, 100.0]),
        ("translate(1e2, -2.5E1)", &[100.0, -25.0]),
    ];
    for (text, want) in cases {
        let got = numbers_in(text);
        if got != want {
            return Err(format!("{text:?}: expected {want:?}, got {got:?}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn text_with_no_number_gives_none_and_a_lone_minus_is_not_one() -> Result<(), String> {
    for text in ["", "translate()", "M L", "-"] {
        if !numbers_in(text).is_empty() {
            return Err(format!("{text:?} gave {:?}", numbers_in(text)));
        }
    }
    // A minus that joins two numbers starts the second.
    if numbers_in("10-5") != [10.0, -5.0] {
        return Err(format!("{:?}", numbers_in("10-5")));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_nodes_a_name_outputs_to_are_found_and_the_inputs_are_not() -> Result<(), String> {
    let cases: [(&str, &[&str]); 5] = [
        ("n3. Output to n5. Input from n1.", &["n5"]),
        ("n3. Output to n5. Output to n9. Input from n1.", &["n5", "n9"]),
        ("solo. Output to blocker. Output to blocker.", &["blocker", "blocker"]),
        ("n7. Input from n2.", &[]),
        ("n7", &[]),
    ];
    for (name, want) in cases {
        let got = outputs_in(name);
        if got != want {
            return Err(format!("{name:?}: expected {want:?}, got {got:?}"));
        }
    }
    Ok(())
}
