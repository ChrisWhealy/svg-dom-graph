//! Huffman encoding, shown in two passes over the same text.
//!
//! The first pass counts how often each byte occurs and builds the code tree. The second pass replaces every byte with
//! its code and packs those odd-length bit sequences into whole output bytes.
//!
//! This increment holds the algorithm only. The scene that walks through it comes later.

#[allow(dead_code)] // Until the walk module consumes it
mod algorithm;
