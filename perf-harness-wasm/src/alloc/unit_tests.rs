use super::*;

fn layout(bytes: usize) -> Layout {
    Layout::from_size_align(bytes, 8).expect("a valid layout")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// These tests call a private instance directly, so they count only their own blocks, whatever else the test run allocates.
#[test]
fn an_allocation_is_counted_and_a_release_gives_it_back_exactly() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let block = unsafe { counter.alloc(layout(1000)) };
    let held = counter.snapshot();
    unsafe { counter.dealloc(block, layout(1000)) };
    let released = counter.snapshot();
    if (held.live_bytes, held.live_allocations, held.total_allocations) != (1000, 1, 1) {
        return Err(format!("{held:?}"));
    }
    if (released.live_bytes, released.live_allocations) != (0, 0)
        || released.total_allocations != 1
        || released.peak_live_bytes != 1000
    {
        return Err(format!("{released:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_zeroed_allocation_counts_like_any_other() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let block = unsafe { counter.alloc_zeroed(layout(64)) };
    let zeroed = (0..64).all(|offset| unsafe { *block.add(offset) } == 0);
    unsafe { counter.dealloc(block, layout(64)) };
    if !zeroed || counter.snapshot().live_bytes != 0 || counter.snapshot().total_allocations != 1 {
        return Err(format!("{:?} zeroed: {zeroed}", counter.snapshot()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A reallocation is the same block under another size. It changes the bytes and not the number of blocks.
#[test]
fn a_reallocation_changes_the_bytes_and_not_the_number_of_blocks() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let mut block = unsafe { counter.alloc(layout(100)) };
    block = unsafe { counter.realloc(block, layout(100), 4000) };
    let grown = counter.snapshot();
    block = unsafe { counter.realloc(block, layout(4000), 50) };
    let shrunk = counter.snapshot();
    unsafe { counter.dealloc(block, layout(50)) };
    let freed = counter.snapshot();
    if (grown.live_bytes, grown.live_allocations, grown.total_allocations) != (4000, 1, 1) {
        return Err(format!("grown {grown:?}"));
    }
    if (shrunk.live_bytes, shrunk.live_allocations, shrunk.peak_live_bytes) != (50, 1, 4000) {
        return Err(format!("shrunk {shrunk:?}"));
    }
    if (freed.live_bytes, freed.live_allocations) != (0, 0) {
        return Err(format!("freed {freed:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_failed_allocation_is_not_counted() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let too_big = Layout::from_size_align(isize::MAX as usize - 64, 1).map_err(|e| e.to_string())?;
    let block = unsafe { counter.alloc(too_big) };
    if !block.is_null() {
        unsafe { counter.dealloc(block, too_big) };
        return Err("the system allocator satisfied an allocation of half the address space".to_string());
    }
    if counter.snapshot()
        != (AllocStats {
            live_bytes: 0,
            live_allocations: 0,
            total_allocations: 0,
            peak_live_bytes: 0,
        })
    {
        return Err(format!("{:?}", counter.snapshot()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Many blocks held at once: the counts add up exactly, and releasing them in another order empties the count.
#[test]
fn many_blocks_add_up_exactly_and_release_in_any_order() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let blocks: Vec<(*mut u8, usize)> = (1..=100).map(|n| (unsafe { counter.alloc(layout(n * 10)) }, n * 10)).collect();
    let total: usize = blocks.iter().map(|(_, size)| size).sum();
    let held = counter.snapshot();
    if held.live_bytes != total || held.live_allocations != 100 {
        return Err(format!("{held:?} against {total}"));
    }
    for (pointer, size) in blocks.into_iter().rev() {
        unsafe { counter.dealloc(pointer, layout(size)) };
    }
    let freed = counter.snapshot();
    if freed.live_bytes != 0 || freed.live_allocations != 0 || freed.peak_live_bytes != total {
        return Err(format!("{freed:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_native_build_does_not_install_the_counter() -> Result<(), String> {
    // On a native target the test harness allocates freely, so nothing has been counted by the global instance.
    if cfg!(not(target_arch = "wasm32")) && stats().total_allocations != 0 {
        return Err(format!("{:?}", stats()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_tracker_lists_the_blocks_allocated_since_it_began_that_are_still_live_with_their_tags() -> Result<(), String> {
    let counter = CountingAllocator::new();
    // A block from before tracking is never listed, whether or not it is freed during it.
    let before = unsafe { counter.alloc(layout(500)) };
    counter.start_tracking();
    counter.set_tag(1);
    let kept = unsafe { counter.alloc(layout(100)) };
    counter.set_tag(2);
    let freed = unsafe { counter.alloc(layout(200)) };
    let also_kept = unsafe { counter.alloc(layout(300)) };
    unsafe { counter.dealloc(freed, layout(200)) };
    unsafe { counter.dealloc(before, layout(500)) };
    counter.stop_tracking();
    let mut live = counter.tracked_live();
    live.sort_by_key(|b| b.size);
    unsafe { counter.dealloc(kept, layout(100)) };
    unsafe { counter.dealloc(also_kept, layout(300)) };
    if live != [TrackedBlock { size: 100, tag: 1 }, TrackedBlock { size: 300, tag: 2 }] {
        return Err(format!("{live:?}"));
    }
    if !counter.tracked_live().is_empty() {
        return Err("a block that was freed after tracking stopped stayed in the list".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_reallocated_block_stays_listed_under_its_new_size_and_its_first_tag() -> Result<(), String> {
    let counter = CountingAllocator::new();
    counter.start_tracking();
    counter.set_tag(7);
    let mut block = unsafe { counter.alloc(layout(64)) };
    counter.set_tag(9);
    block = unsafe { counter.realloc(block, layout(64), 4096) };
    counter.stop_tracking();
    let live = counter.tracked_live();
    unsafe { counter.dealloc(block, layout(4096)) };
    if live != [TrackedBlock { size: 4096, tag: 7 }] {
        return Err(format!("{live:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// More blocks than the table has slots: the extra are counted as untracked, nothing is lost silently, and the counts of the
/// allocator itself are not affected.
#[test]
fn a_full_table_says_how_many_blocks_it_could_not_follow() -> Result<(), String> {
    let counter = CountingAllocator::new();
    counter.start_tracking();
    let blocks: Vec<*mut u8> = (0..SLOTS + 50).map(|_| unsafe { counter.alloc(layout(16)) }).collect();
    counter.stop_tracking();
    let (followed, missed) = (counter.tracked_live().len(), counter.untracked());
    let live = counter.snapshot().live_allocations;
    for block in blocks {
        unsafe { counter.dealloc(block, layout(16)) };
    }
    if followed != SLOTS || missed != 50 || live != SLOTS + 50 {
        return Err(format!("{followed} followed, {missed} missed, {live} live"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Freed slots are reused, so a long run that allocates and frees a great deal does not fill the table.
#[test]
fn freed_slots_are_reused() -> Result<(), String> {
    let counter = CountingAllocator::new();
    counter.start_tracking();
    for _ in 0..(SLOTS * 5) {
        let block = unsafe { counter.alloc(layout(32)) };
        unsafe { counter.dealloc(block, layout(32)) };
    }
    counter.stop_tracking();
    if counter.untracked() != 0 || !counter.tracked_live().is_empty() {
        return Err(format!(
            "{} untracked, {} listed",
            counter.untracked(),
            counter.tracked_live().len()
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A block from before tracking that grows during it is otherwise an allocation that no list shows, so it is followed from the
/// reallocation on, under a tag that says what it is.
#[test]
fn a_block_from_before_tracking_that_grows_is_followed_under_its_own_tag() -> Result<(), String> {
    let counter = CountingAllocator::new();
    let mut block = unsafe { counter.alloc(layout(100)) };
    counter.start_tracking();
    counter.set_tag(3);
    block = unsafe { counter.realloc(block, layout(100), 612) };
    counter.stop_tracking();
    let live = counter.tracked_live();
    unsafe { counter.dealloc(block, layout(612)) };
    if live != [TrackedBlock { size: 612, tag: GROWN_FLAG | 3 }] {
        return Err(format!("{live:?}"));
    }
    Ok(())
}
