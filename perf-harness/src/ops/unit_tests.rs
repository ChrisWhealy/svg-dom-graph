use super::*;

/// A short stream that uses every operation. Its two digests were produced by a separate implementation in Python, from
/// the layout in `encode`.
fn sample() -> Vec<Op> {
    let node = |key: &str, x: i64, y: i64| Op::AddNode {
        key: key.into(),
        x,
        y,
        w: 80,
        h: 36,
        label: key.into(),
    };
    vec![
        Op::CreateScene,
        node("a", 1, 2),
        node("b", -5, 7),
        Op::AddEdge {
            from: "a".into(),
            to: "b".into(),
        },
        Op::MoveNode { key: "a".into(), x: -5, y: 7 },
        Op::Phase { phase: Phase::Measure },
        Op::Checkpoint { index: 0 },
        Op::DropScene,
        Op::RemoveHost,
    ]
}

fn hash(ops: &[Op]) -> (String, String) {
    let mut hasher = StreamHasher::new();
    for op in ops {
        hasher.push(op);
    }
    hasher.finish()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_stream_digests_match_an_independent_implementation() -> Result<(), String> {
    let (full, mutations) = hash(&sample());
    if full != "afc0904a6c4281ea1d0998ee056f68f694c9d33a3f73f325f270d6818283f87f" {
        return Err(format!("full digest {full}"));
    }
    if mutations != "e81e0101b1b8731ff6c6562eaa3592b233e117aefe27ecf7f738b0b630fd614f" {
        return Err(format!("mutations digest {mutations}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn markers_change_the_full_digest_and_not_the_mutations_digest() -> Result<(), String> {
    let ops = sample();
    let without_markers: Vec<Op> = ops.iter().filter(|op| op.is_mutation()).cloned().collect();
    let (full, mutations) = hash(&ops);
    let (stripped_full, stripped_mutations) = hash(&without_markers);
    if mutations != stripped_mutations {
        return Err("taking out the markers changed the mutations digest".to_string());
    }
    if full == stripped_full {
        return Err("taking out the markers left the full digest as it was".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn changing_any_one_operation_changes_both_digests() -> Result<(), String> {
    let base = hash(&sample());
    let mut moved = sample();
    moved[4] = Op::MoveNode { key: "a".into(), x: -5, y: 8 };
    let changed = hash(&moved);
    if changed.0 == base.0 || changed.1 == base.1 {
        return Err("a changed move left a digest as it was".to_string());
    }
    let mut reordered = sample();
    reordered.swap(1, 2);
    let swapped = hash(&reordered);
    if swapped.0 == base.0 || swapped.1 == base.1 {
        return Err("a changed order left a digest as it was".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_counts_say_how_many_operations_and_how_many_mutations() -> Result<(), String> {
    let mut hasher = StreamHasher::new();
    for op in sample() {
        hasher.push(&op);
    }
    if hasher.operations() != 9 || hasher.mutation_count() != 7 {
        return Err(format!(
            "{} operations and {} mutations",
            hasher.operations(),
            hasher.mutation_count()
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_string_cannot_be_mistaken_for_its_neighbour() -> Result<(), String> {
    // Without a length in front of each string, ("ab", "c") and ("a", "bc") would encode alike.
    let first = hash(&[Op::AddEdge {
        from: "ab".into(),
        to: "c".into(),
    }]);
    let second = hash(&[Op::AddEdge {
        from: "a".into(),
        to: "bc".into(),
    }]);
    if first == second {
        Err("two different edges hashed alike".to_string())
    } else {
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The hash keeps one buffer that settles at the size of the largest operation, and that is a 64-byte block that stays allocated
/// for the life of a run. It is the one block that an endurance run's attribution finds that the harness itself owns.
#[test]
fn the_encoder_buffer_settles_at_64_bytes_for_the_largest_operation_of_a_workload() -> Result<(), String> {
    let mut encoder = Encoder::new();
    let widest = Op::AddNode {
        key: "n39".into(),
        x: 4000,
        y: 2000,
        w: 80,
        h: 36,
        label: "n39".into(),
    };
    widest.encode(&mut encoder);
    // One code byte, two 8-byte strings with a tag and a length (5 + 3 each), and four 9-byte integers.
    if encoder.as_slice().len() != 1 + 2 * 8 + 4 * 9 || encoder.capacity() != 64 {
        return Err(format!("{} bytes, capacity {}", encoder.as_slice().len(), encoder.capacity()));
    }
    // A smaller operation afterwards does not shrink it.
    Op::CreateScene.encode(&mut encoder);
    if encoder.capacity() == 64 {
        Ok(())
    } else {
        Err(format!("capacity {}", encoder.capacity()))
    }
}
