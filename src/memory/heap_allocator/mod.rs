use alloc::heap::{Alloc, AllocErr, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

pub mod linked_list_allocator;

#[derive(Debug)]
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new(heap_start: usize, heap_end: usize) -> BumpAllocator {
        BumpAllocator { heap_start, heap_end, next: AtomicUsize::new(heap_start) }
    }
}

unsafe impl<'a> Alloc for &'a BumpAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        loop {
            let current_next = self.next.load(Ordering::Relaxed);
            let alloc_start = super::align_up(current_next, layout.align());
            let alloc_end = alloc_start.saturating_add(layout.size());

            if alloc_end <= self.heap_end {
                let next_now = self.next.compare_and_swap(current_next, alloc_end, Ordering::Relaxed);
                if next_now == current_next {
                    return Ok(alloc_start as *mut u8);
                }
            }
                else {
                    return Err(AllocErr::Exhausted { request: layout })
                }
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        // Leak memory (for now...)
    }

    fn oom(&mut self, _: AllocErr) -> ! {
        panic!("Out of memory!");
    }
}