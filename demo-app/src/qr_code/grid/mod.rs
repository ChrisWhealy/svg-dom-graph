//! The matrix of a QR symbol: what colour each module is, and what each module is for.
//!
//! Colour and role are separate. A module can hold a role before it holds a colour: the format modules are reserved
//! when the function patterns are drawn, but their colours are not known until a mask is chosen. A data slot holds
//! neither until a codeword is placed in it. Every mutation checks its rules first and then writes, so a rejected call
//! leaves the grid exactly as it was.
//!
//! The types that have methods of their own are each in a file of their own: [`BitGrid`], the matrix itself, in
//! `bit_grid.rs`, [`QrModuleKind`], what a module is for, in `qr_module_kind.rs`, and [`GridError`], why a change was
//! refused, in `grid_error.rs`. This file holds the plain types they are made of.

mod bit_grid;
mod grid_error;
mod qr_module_kind;

pub(crate) use bit_grid::BitGrid;
pub(crate) use grid_error::GridError;
pub(crate) use qr_module_kind::QrModuleKind;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The width and height of a Version 1 symbol, in modules. [`BitGrid`] does not use it, and only callers that build a
/// symbol do.
pub(crate) const SIZE: usize = 21;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The colour of one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellState {
    /// No colour has been chosen yet. This is not the same as light.
    Unassigned,
    Light,
    Dark,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A module that belongs to a fixed pattern, not to the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionRole {
    Finder,
    Separator,
    Timing,
    DarkModule,
    Format,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A module that holds one bit of a codeword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataRole {
    Data,
    ErrorCorrection,
}
