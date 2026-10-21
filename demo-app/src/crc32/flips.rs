//! The bits the user has flipped between the sender and the receiver, in the data and in the CRC that is sent with it.
//! Nothing here touches the DOM or the library.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which bits are flipped. Flipping a bit that is already flipped puts it back, so each bit is flipped or not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Flips {
    /// Bits of the data: the byte, and the bit in it, counting from 0 at the least significant.
    /// In order, with no repeats.
    pub(super) data: Vec<(usize, u8)>,
    /// Bits of the CRC, counting from 0 at the least significant. In order, with no repeats.
    pub(super) crc: Vec<u8>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most flipped bits that the status line names. More than this are counted.
const MOST_NAMED: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Flips {
    /// Whether no bit is flipped.
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.data.is_empty() && self.crc.is_empty()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many bits go through the channel for `len` bytes of data: every bit of the data, and the 32 of the CRC.
    pub(super) fn bits_in(len: usize) -> usize {
        len * 8 + 32
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `count` different bits chosen at random from the data's `len` bytes and the CRC, which is as many as there are
    /// if `count` is too large. `next` gives a number from 0 up to but not including 1 each time it is called.
    pub(super) fn random(count: usize, len: usize, mut next: impl FnMut() -> f64) -> Self {
        let total = Self::bits_in(len);
        let mut pool: Vec<usize> = (0..total).collect();
        let count = count.min(total);
        // A partial shuffle: each pick is from what is left, and lands at the front.
        for taken in 0..count {
            let pick = taken + ((next() * (total - taken) as f64) as usize).min(total - taken - 1);
            pool.swap(taken, pick);
        }
        let mut flips = Self::default();
        for &index in &pool[..count] {
            if index < len * 8 {
                flips.data.push((index / 8, (index % 8) as u8));
            } else {
                flips.crc.push((index - len * 8) as u8);
            }
        }
        flips.data.sort_unstable();
        flips.crc.sort_unstable();
        flips
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Forgets every flip in a byte at or past `len`, as when the text gets shorter.
    pub(super) fn keep_bytes_below(&mut self, len: usize) {
        self.data.retain(|&(byte, _)| byte < len);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The flipped bits of the CRC as a mask to XOR it with.
    pub(super) fn crc_mask(&self) -> u32 {
        self.crc.iter().fold(0, |mask, &bit| mask ^ (1 << bit))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The bytes with a flipped bit in them, in order, each once.
    pub(super) fn bytes(&self) -> Vec<usize> {
        let mut bytes: Vec<usize> = self.data.iter().map(|&(byte, _)| byte).collect();
        bytes.dedup();
        bytes
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the status line says about them. A few are named. Many are counted, since a list of them would not fit.
    pub(super) fn describe(&self) -> String {
        let count = self.data.len() + self.crc.len();
        if count > MOST_NAMED {
            return format!(
                "{count} bits are flipped in transmission: {} of the data and {} of the CRC.",
                self.data.len(),
                self.crc.len()
            );
        }
        let places: Vec<String> = self
            .data
            .iter()
            .map(|&(byte, bit)| format!("bit {bit} of byte {byte}"))
            .chain(self.crc.iter().map(|bit| format!("bit {bit} of the CRC")))
            .collect();
        match places.as_slice() {
            [] => "No error has been introduced.".to_owned(),
            [only] => {
                let mut sentence = format!("{only} is flipped in transmission.");
                sentence[..1].make_ascii_uppercase();
                sentence
            },
            _ => format!("Flipped in transmission: {}.", places.join(", ")),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests {
    use super::Flips;

    /// A "random" source that counts up through 0 to just under 1 and wraps, so the picks are known.
    fn counting(step: f64) -> impl FnMut() -> f64 {
        let mut at = 0.0;
        move || {
            let now = at;
            at = (at + step) % 1.0;
            now
        }
    }

    #[test]
    fn random_flips_are_the_number_asked_for_in_order_with_no_repeats() {
        for count in [1, 2, 7, 50, 80] {
            let flips = Flips::random(count, 6, counting(0.137));
            assert_eq!(flips.data.len() + flips.crc.len(), count);
            let mut data = flips.data.clone();
            data.dedup();
            assert_eq!(data, flips.data);
            assert!(flips.data.windows(2).all(|w| w[0] < w[1]));
            assert!(flips.crc.windows(2).all(|w| w[0] < w[1]));
            assert!(flips.data.iter().all(|&(byte, bit)| byte < 6 && bit < 8));
            assert!(flips.crc.iter().all(|&bit| bit < 32));
        }
    }

    #[test]
    fn asking_for_every_bit_flips_every_bit_and_asking_for_more_is_the_same() {
        let all = Flips::random(Flips::bits_in(3), 3, counting(0.31));
        assert_eq!(all.data.len(), 24);
        assert_eq!(all.crc.len(), 32);
        assert_eq!(Flips::random(1000, 3, counting(0.31)), all);
    }

    #[test]
    fn a_source_at_either_end_of_its_range_still_picks_a_bit_that_exists() {
        for value in [0.0, 0.999_999_999_999] {
            let flips = Flips::random(5, 2, || value);
            assert_eq!(flips.data.len() + flips.crc.len(), 5);
        }
    }

    #[test]
    fn the_crc_mask_has_a_one_for_each_flipped_bit() {
        let flips = Flips {
            data: Vec::new(),
            crc: vec![0, 4, 31],
        };
        assert_eq!(flips.crc_mask(), 0x8000_0011);
    }

    #[test]
    fn a_shorter_text_forgets_the_flips_it_no_longer_has() {
        let mut flips = Flips {
            data: vec![(1, 1), (9, 2)],
            crc: Vec::new(),
        };
        flips.keep_bytes_below(5);
        assert_eq!(flips.data, vec![(1, 1)]);
        assert!(!flips.is_empty());
    }

    #[test]
    fn the_bytes_with_a_flipped_bit_are_listed_once_each() {
        let flips = Flips {
            data: vec![(2, 1), (2, 7), (5, 3)],
            crc: Vec::new(),
        };
        assert_eq!(flips.bytes(), vec![2, 5]);
    }

    #[test]
    fn the_status_line_names_a_few_flipped_bits_and_counts_many() {
        let mut flips = Flips::default();
        assert_eq!(flips.describe(), "No error has been introduced.");
        flips.data = vec![(2, 5)];
        assert_eq!(flips.describe(), "Bit 5 of byte 2 is flipped in transmission.");
        flips.data.clear();
        flips.crc = vec![12];
        assert_eq!(flips.describe(), "Bit 12 of the CRC is flipped in transmission.");
        flips.data = vec![(7, 0)];
        assert_eq!(flips.describe(), "Flipped in transmission: bit 0 of byte 7, bit 12 of the CRC.");
        flips.data = vec![(1, 0), (2, 0), (3, 0), (4, 0)];
        assert_eq!(
            flips.describe(),
            "5 bits are flipped in transmission: 4 of the data and 1 of the CRC."
        );
    }
}
