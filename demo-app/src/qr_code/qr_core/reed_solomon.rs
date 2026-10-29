//! Reed-Solomon error correction codewords: the remainder of the data, as a polynomial, divided by a generator.
//!
//! A byte is a coefficient of GF(2^8), and the first byte is the coefficient of the highest power. The generator of
//! degree `n` is the product of `x - alpha^i` for `i` from 0 to `n - 1`. Its leading coefficient is 1, so the division
//! needs no inverse. The remainder has `n` bytes, and they are the error correction codewords.

use super::gf256::Gf256;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The generator polynomial of degree `degree`, highest power first. It has `degree + 1` coefficients, and the first is
/// 1.
pub(crate) fn generator_polynomial(degree: usize) -> Vec<Gf256> {
    let mut generator = vec![Gf256::ONE];
    for exponent in 0..degree {
        // Multiply by (x + alpha^exponent): the old coefficients move up one power, and are added to the old ones times
        // alpha^exponent.
        let root = Gf256::alpha_pow(exponent);
        let mut next = vec![Gf256::ZERO; generator.len() + 1];
        for (i, &coefficient) in generator.iter().enumerate() {
            next[i] = next[i] + coefficient;
            next[i + 1] = next[i + 1] + coefficient * root;
        }
        generator = next;
    }
    generator
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One iteration of the long division: one data byte taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DivisionStep {
    /// The data byte this iteration took in.
    pub(crate) data_byte: u8,
    /// The data byte added to the first byte of the register. It is the multiple of the generator to subtract.
    pub(crate) factor: u8,
    /// The remainder so far, `degree` bytes. After the last iteration it is the error correction codewords.
    pub(crate) register: Vec<u8>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Divides `data`, followed by `degree` zeros, by the generator of degree `degree`, one data byte at a time.
///
/// Dividing the data and then the zeros is the same as dividing the data in a register of `degree` bytes: each byte
/// shifts the register up, and the byte that falls out, added to the data byte, says how much of the generator to
/// subtract from what is left. So there is one step for each data byte, and none for the zeros.
pub(crate) fn divide(data: &[u8], degree: usize) -> Vec<DivisionStep> {
    let generator = generator_polynomial(degree);
    let mut register = vec![Gf256::ZERO; degree];
    let mut steps = Vec::with_capacity(data.len());
    for &data_byte in data {
        let factor = Gf256::new(data_byte) + register.first().copied().unwrap_or(Gf256::ZERO);
        register.rotate_left(1);
        if let Some(last) = register.last_mut() {
            *last = Gf256::ZERO;
        }
        for (cell, &coefficient) in register.iter_mut().zip(&generator[1..]) {
            *cell = *cell + coefficient * factor;
        }
        steps.push(DivisionStep {
            data_byte,
            factor: factor.value(),
            register: register.iter().map(|cell| cell.value()).collect(),
        });
    }
    steps
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `degree` error correction bytes for `data`, which are the register after the last step.
pub(crate) fn remainder(steps: &[DivisionStep], degree: usize) -> Vec<u8> {
    steps.last().map_or_else(|| vec![0; degree], |step| step.register.clone())
}
