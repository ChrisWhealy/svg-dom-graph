use super::{Newest, Walk};

#[test]
fn a_text_has_eight_more_positions_than_it_has_bytes() {
    assert_eq!(Walk::new(1).positions(), 9);
    assert_eq!(Walk::new(49).positions(), 57);
    assert_eq!(Walk::new(64).positions(), 72);
}

#[test]
fn no_text_has_the_initial_register_and_the_crc() {
    let walk = Walk::new(0);
    assert_eq!(walk.positions(), 2);
    assert_eq!(walk.progress(0).newest, Newest::LastRegister);
    assert!(walk.progress(0).last_register && !walk.progress(0).crc);
    assert_eq!(walk.progress(1).newest, Newest::Crc);
    assert!(walk.progress(1).crc);
    // Nothing is sent, so nothing arrives.
    assert!(!walk.progress(1).arrived);
    assert_eq!(walk.arrival(), 1);
}

#[test]
fn the_first_position_has_reached_nothing() {
    let first = Walk::new(5).progress(0);
    assert_eq!(first.newest, Newest::Nothing);
    assert_eq!(first.bytes_done, 0);
    assert!(!first.last_register && !first.crc && !first.arrived && !first.corrected);
}

#[test]
fn each_byte_has_a_position_of_its_own() {
    let walk = Walk::new(5);
    for byte in 0..5 {
        let progress = walk.progress(byte + 1);
        assert_eq!(progress.newest, Newest::Byte(byte));
        assert_eq!(progress.bytes_done, byte + 1);
    }
    // The register after the last byte is reached with the last byte, and the CRC one step later.
    assert!(walk.progress(5).last_register && !walk.progress(5).crc);
    assert_eq!(walk.progress(6).newest, Newest::Crc);
    assert!(walk.progress(6).crc && !walk.progress(6).arrived);
}

#[test]
fn the_receiver_is_reached_one_box_at_a_time_in_order() {
    let walk = Walk::new(5);
    let order = [
        Newest::Arrival,
        Newest::ReceiverRegisters,
        Newest::ReceivedCrc,
        Newest::Syndrome,
        Newest::Position,
        Newest::Corrected,
    ];
    for (step, newest) in order.into_iter().enumerate() {
        assert_eq!(walk.progress(7 + step).newest, newest);
    }
    assert_eq!(walk.arrival(), 7);
    let end = walk.progress(walk.positions() - 1);
    assert!(end.arrived && end.receiver_registers && end.received_crc && end.syndrome && end.position && end.corrected);
}

#[test]
fn nothing_that_was_reached_is_lost_by_stepping_on() {
    let walk = Walk::new(3);
    let flags = |p| {
        let g = walk.progress(p);
        [
            g.last_register,
            g.crc,
            g.arrived,
            g.receiver_registers,
            g.received_crc,
            g.syndrome,
            g.position,
            g.corrected,
        ]
    };
    for p in 1..walk.positions() {
        for (before, after) in flags(p - 1).into_iter().zip(flags(p)) {
            assert!(after || !before, "position {p} lost a box");
        }
        assert!(walk.progress(p).bytes_done >= walk.progress(p - 1).bytes_done);
    }
}

#[test]
fn a_position_past_the_end_is_the_end() {
    let walk = Walk::new(3);
    assert_eq!(walk.progress(1000), walk.progress(walk.positions() - 1));
}
