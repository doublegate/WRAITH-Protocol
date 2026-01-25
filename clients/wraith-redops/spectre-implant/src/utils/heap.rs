use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

// A simple bump allocator for no_std environments
pub struct MiniHeap {
    heap_start: UnsafeCell<usize>,
    heap_end: usize,
    current_pos: UnsafeCell<usize>,
}

unsafe impl Sync for MiniHeap {}

impl MiniHeap {
    pub const fn new(start: usize, size: usize) -> Self {
        Self {
            heap_start: UnsafeCell::new(start),
            heap_end: start + size,
            current_pos: UnsafeCell::new(start),
        }
    }
}

unsafe impl GlobalAlloc for MiniHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let current = *self.current_pos.get();
        let align = layout.align();
        let size = layout.size();

        // Calculate alignment padding
        let padding = (align - (current % align)) % align;
        let start = current + padding;
        let end = start + size;

        if end > self.heap_end {
            core::ptr::null_mut() // OOM
        } else {
            *self.current_pos.get() = end;
            start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator cannot free individual items
        // Resetting the whole heap would be done manually if needed
    }
}
