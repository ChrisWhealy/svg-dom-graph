// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The byte order `DataNodeContent::cells` splits each value into, for [`super::DataFormat::Hexadecimal`]/
/// [`super::DataFormat::Binary`]. See this module's own doc comment ("Formatting") for why `BigEndian` is the default, and
/// when a caller wants `LittleEndian` instead.
///
/// Has no visible effect under [`super::DataFormat::Decimal`]. A plain decimal number reads the same regardless of which byte
/// order produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ByteOrder {
    /// Most significant byte first, independent of host endianness. So the displayed digits always read the same way a
    /// human would write the number, on every host. The right choice for register/value display.
    #[default]
    BigEndian,
    /// Least significant byte first. The right choice when a value's own byte order is a property of the data being
    /// inspected — an actual in-memory layout — rather than a display preference.
    LittleEndian,
}
