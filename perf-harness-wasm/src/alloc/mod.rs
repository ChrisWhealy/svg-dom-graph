//! A global allocator that counts, to say how much Rust memory is live.
//!
//! The size of the wasm linear memory cannot say that. It grows in pages and never shrinks, so it is a high-water mark of what
//! was ever needed, and a freed block that a retained one then reuses leaves it unchanged. The JS heap cannot say it either,
//! since this memory is not on it. The library deliberately leaks some closures with `Closure::forget`, and the figure in
//! which that shows is the number of bytes that are live, which this counts.
//!
//! It counts the sizes that were asked for, not what the underlying allocator reserved, so it leaves out that allocator's own
//! overhead. It wraps the system allocator and changes nothing about what is allocated.
//!
//! The counters are plain atomics. A wasm module has one thread, so there is no contention, and a native test of the
//! arithmetic can use a private instance that is not installed as the global one.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// What the allocator has counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocStats {
    /// The bytes that are allocated now.
    pub live_bytes: usize,
    /// The blocks that are allocated now.
    pub live_allocations: usize,
    /// Every block ever allocated, whether or not it has been freed.
    pub total_allocations: usize,
    /// The most that `live_bytes` has been.
    pub peak_live_bytes: usize,
}

#[derive(Debug)]
pub struct CountingAllocator {
    live_bytes: AtomicUsize,
    live_allocations: AtomicUsize,
    total_allocations: AtomicUsize,
    peak_live_bytes: AtomicUsize,
}

impl Default for CountingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            live_bytes: AtomicUsize::new(0),
            live_allocations: AtomicUsize::new(0),
            total_allocations: AtomicUsize::new(0),
            peak_live_bytes: AtomicUsize::new(0),
        }
    }

    pub fn snapshot(&self) -> AllocStats {
        AllocStats {
            live_bytes: self.live_bytes.load(Relaxed),
            live_allocations: self.live_allocations.load(Relaxed),
            total_allocations: self.total_allocations.load(Relaxed),
            peak_live_bytes: self.peak_live_bytes.load(Relaxed),
        }
    }

    fn grew(&self, bytes: usize) {
        let live = self.live_bytes.fetch_add(bytes, Relaxed) + bytes;
        self.peak_live_bytes.fetch_max(live, Relaxed);
    }

    fn counted(&self, pointer: *mut u8, bytes: usize) -> *mut u8 {
        // A failed allocation hands back null, and nothing was allocated.
        if !pointer.is_null() {
            self.grew(bytes);
            self.live_allocations.fetch_add(1, Relaxed);
            self.total_allocations.fetch_add(1, Relaxed);
        }
        pointer
    }
}

// SAFETY: every call is passed on to `System` with the same arguments, so whatever `System` guarantees, this guarantees too.
// The counters are only read and written, and never decide what is returned.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.counted(unsafe { System.alloc(layout) }, layout.size())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.counted(unsafe { System.alloc_zeroed(layout) }, layout.size())
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        self.live_bytes.fetch_sub(layout.size(), Relaxed);
        self.live_allocations.fetch_sub(1, Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        // A failed reallocation leaves the old block where it was, so only a success changes the count. It is the same block
        // under another size, so the number of blocks does not change.
        if !moved.is_null() {
            if new_size >= layout.size() {
                self.grew(new_size - layout.size());
            } else {
                self.live_bytes.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        moved
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The allocator of a wasm build. A native build has the usual allocator, so that a test run is not changed by this crate.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
pub static GLOBAL: CountingAllocator = CountingAllocator::new();

#[cfg(not(target_arch = "wasm32"))]
pub static GLOBAL: CountingAllocator = CountingAllocator::new();

/// What the global allocator has counted. A native build is not counting, so it is all zeros there.
pub fn stats() -> AllocStats {
    GLOBAL.snapshot()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
