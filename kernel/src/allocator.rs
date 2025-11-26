use linked_list_allocator::LockedHeap;

unsafe extern "C" {
    static mut _sheap: u8;
    static mut _eheap: u8;
}

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the heap allocator.
/// Must be called before any heap allocations occur.
pub fn init() {
    unsafe {
        let heap_start = &raw mut _sheap as *mut u8;
        let heap_end = &raw const _eheap as usize;
        let heap_size = heap_end - (heap_start as usize);
        ALLOCATOR.lock().init(heap_start, heap_size);
    }
}

/// Returns (used, free) bytes in the heap, if the allocator supports introspection.
pub fn heap_stats() -> (usize, usize) {
    let allocator = ALLOCATOR.lock();
    let used = allocator.used();
    let free = allocator.free();
    (used, free)
}

/// Returns the total heap size.
pub fn heap_size() -> usize {
    let heap_start = &raw const _sheap as usize;
    let heap_end = &raw const _eheap as usize;
    heap_end - heap_start
}
