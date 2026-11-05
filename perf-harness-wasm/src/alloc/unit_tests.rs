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
