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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

/// How many live blocks the tracker can follow at once. It is a fixed table, since the allocator must never allocate.
const SLOTS: usize = 2048;
/// How many growths of blocks from before tracking are remembered, with their old and new sizes.
const GROWTHS: usize = 32;
/// Added to the tag when a block that was allocated before tracking began is reallocated during it, so that the tag says both
/// that it grew and what the program was doing when it did.
pub const GROWN_FLAG: usize = 128;
/// A slot whose block was freed. Searches pass over it, and a new block may take it.
const FREED: usize = usize::MAX;

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

/// A block that was allocated while tracking was on and is still live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedBlock {
    /// Where the block is in linear memory, which is what lets a diagnostic read what it holds.
    pub address: usize,
    pub size: usize,
    /// What the program was doing when the block was allocated, as the tag it had set. See [`CountingAllocator::set_tag`].
    pub tag: usize,
}

#[derive(Debug)]
pub struct CountingAllocator {
    live_bytes: AtomicUsize,
    live_allocations: AtomicUsize,
    total_allocations: AtomicUsize,
    peak_live_bytes: AtomicUsize,
    // The tracker. When it is on, every block allocated is entered in a table with its size and the current tag, and taken out
    // when it is freed, so that what is left in it is what was allocated since tracking began and has not been freed.
    tracking: AtomicBool,
    /// Whether tracking has ever been turned on. Until it has, freeing a block costs nothing extra, so that a run that never
    /// tracks has no look-up in its timed calls.
    ever_tracked: AtomicBool,
    tag: AtomicUsize,
    dropped: AtomicUsize,
    pointers: [AtomicUsize; SLOTS],
    details: [AtomicUsize; SLOTS],
    grown_count: AtomicUsize,
    grown_old: [AtomicUsize; GROWTHS],
    grown_new: [AtomicUsize; GROWTHS],
    grown_tag: [AtomicUsize; GROWTHS],
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
            tracking: AtomicBool::new(false),
            ever_tracked: AtomicBool::new(false),
            tag: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
            pointers: [const { AtomicUsize::new(0) }; SLOTS],
            details: [const { AtomicUsize::new(0) }; SLOTS],
            grown_count: AtomicUsize::new(0),
            grown_old: [const { AtomicUsize::new(0) }; GROWTHS],
            grown_new: [const { AtomicUsize::new(0) }; GROWTHS],
            grown_tag: [const { AtomicUsize::new(0) }; GROWTHS],
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Starts following every block that is allocated from now on.
    pub fn start_tracking(&self) {
        self.ever_tracked.store(true, Relaxed);
        self.tracking.store(true, Relaxed);
    }

    /// Stops following new blocks. A block that was followed is still taken out of the table when it is freed.
    pub fn stop_tracking(&self) {
        self.tracking.store(false, Relaxed);
    }

    /// Sets what the program is doing, so that the blocks allocated from now on carry it.
    pub fn set_tag(&self, tag: usize) {
        self.tag.store(tag, Relaxed);
    }

    /// How many blocks could not be followed because the table was full. Zero means that the list is complete.
    pub fn untracked(&self) -> usize {
        self.dropped.load(Relaxed)
    }

    /// The blocks that were allocated while tracking was on and have not been freed. This allocates, so stop tracking first.
    pub fn tracked_live(&self) -> Vec<TrackedBlock> {
        (0..SLOTS)
            .filter_map(|slot| {
                let pointer = self.pointers[slot].load(Relaxed);
                (pointer != 0 && pointer != FREED).then(|| {
                    let detail = self.details[slot].load(Relaxed);
                    TrackedBlock {
                        address: pointer,
                        size: detail >> 8,
                        tag: detail & 0xff,
                    }
                })
            })
            .collect()
    }

    /// The growths of blocks from before tracking that happened during it, as `(old size, new size, tag)`, oldest first.
    pub fn growths(&self) -> Vec<(usize, usize, usize)> {
        let count = self.grown_count.load(Relaxed).min(GROWTHS);
        (0..count)
            .map(|at| {
                (
                    self.grown_old[at].load(Relaxed),
                    self.grown_new[at].load(Relaxed),
                    self.grown_tag[at].load(Relaxed),
                )
            })
            .collect()
    }

    fn slot_of(pointer: usize) -> usize {
        (pointer >> 4) % SLOTS
    }

    fn track(&self, pointer: *mut u8, size: usize, tag: usize) {
        let address = pointer as usize;
        let start = Self::slot_of(address);
        for step in 0..SLOTS {
            let slot = (start + step) % SLOTS;
            let held = self.pointers[slot].load(Relaxed);
            if (held == 0 || held == FREED)
                && self.pointers[slot].compare_exchange(held, address, Relaxed, Relaxed).is_ok()
            {
                self.details[slot].store((size << 8) | (tag & 0xff), Relaxed);
                return;
            }
        }
        self.dropped.fetch_add(1, Relaxed);
    }

    /// Takes a block out of the table, and gives back what it was recorded with, if it was in it.
    fn untrack(&self, pointer: *mut u8) -> Option<usize> {
        if !self.ever_tracked.load(Relaxed) {
            return None;
        }
        let address = pointer as usize;
        let start = Self::slot_of(address);
        for step in 0..SLOTS {
            let slot = (start + step) % SLOTS;
            match self.pointers[slot].load(Relaxed) {
                0 => return None,
                held if held == address => {
                    self.pointers[slot].store(FREED, Relaxed);
                    return Some(self.details[slot].load(Relaxed));
                },
                _ => {},
            }
        }
        None
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
            if self.tracking.load(Relaxed) {
                self.track(pointer, bytes, self.tag.load(Relaxed));
            }
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
        self.untrack(pointer);
        self.live_bytes.fetch_sub(layout.size(), Relaxed);
        self.live_allocations.fetch_sub(1, Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        // A failed reallocation leaves the old block where it was, so only a success changes the count. It is the same block
        // under another size, so the number of blocks does not change.
        if !moved.is_null() {
            // A block that was followed stays followed under its new address and size, with the tag it began with.
            match self.untrack(pointer) {
                Some(detail) => self.track(moved, new_size, detail & 0xff),
                // A block from before tracking began that grew. It is followed from here under a tag of its own, since what
                // it grew by is otherwise an allocation that no list shows. The tag says what the program was doing.
                None if self.tracking.load(Relaxed) => {
                    let tag = self.tag.load(Relaxed) & 0x7f;
                    let at = self.grown_count.fetch_add(1, Relaxed);
                    if at < GROWTHS {
                        self.grown_old[at].store(layout.size(), Relaxed);
                        self.grown_new[at].store(new_size, Relaxed);
                        self.grown_tag[at].store(tag, Relaxed);
                    }
                    self.track(moved, new_size, GROWN_FLAG | tag);
                },
                None => {},
            }
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(not(target_arch = "wasm32"))]
pub static GLOBAL: CountingAllocator = CountingAllocator::new();

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Starts, stops and tags the tracker of the global allocator. See [`CountingAllocator`].
pub fn start_tracking() {
    GLOBAL.start_tracking();
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn stop_tracking() {
    GLOBAL.stop_tracking();
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn set_tag(tag: usize) {
    GLOBAL.set_tag(tag);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn untracked() -> usize {
    GLOBAL.untracked()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn tracked_live() -> Vec<TrackedBlock> {
    GLOBAL.tracked_live()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn growths() -> Vec<(usize, usize, usize)> {
    GLOBAL.growths()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the global allocator has counted. A native build is not counting, so it is all zeros there.
pub fn stats() -> AllocStats {
    GLOBAL.snapshot()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
