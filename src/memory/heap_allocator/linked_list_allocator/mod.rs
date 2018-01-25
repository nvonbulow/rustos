use core::mem;
use core::ops::Deref;
use alloc::allocator::{Alloc, Layout, AllocErr};
use spin::Mutex;

use memory::align_up;
use self::hole::{Hole, HoleList};

mod hole;

pub struct Heap {
    bottom: usize,
    size: usize,
    holes: HoleList,
}

impl Heap {
    // Empty heap where all calls will return `None`
    pub const fn empty() -> Heap {
        Heap {
            bottom: 0,
            size: 0,
            holes: HoleList::empty(),
        }
    }

    // Initialize an empty heap
    // Must be called at most once and only on an empty heap
    pub unsafe fn init(&mut self, heap_bottom: usize, heap_size: usize) {
        self.bottom = heap_bottom;
        self.size = heap_size;
        self.holes = HoleList::new(heap_bottom, heap_size);
    }

    // Creates a new heap with the given bottom and size
    // Bottom address must be valid and memory in the [heap_bottom, heap_bottom + heap_size) range
    // must be completely free
    pub unsafe fn new(heap_bottom: usize, heap_size: usize) -> Heap {
        Heap {
            bottom: heap_bottom,
            size: heap_size,
            holes: HoleList::new(heap_bottom, heap_size),
        }
    }

    // Allocates a chunk of the given size with the given alignment
    // Returns a pointer to the beginning of that chunk if it was successful else `None`
    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size, mem::align_of::<Hole>());
        let layout = Layout::from_size_align(size, layout.align()).unwrap();

        self.holes.allocate_first_fit(layout)
    }

    // Frees the given allocation. `ptr` must be a pointer returned by `allocate_first_fit`
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let mut size = layout.size();
        if size < HoleList::min_size() {
            size = HoleList::min_size();
        }
        let size = align_up(size, mem::align_of::<Hole>());
        let layout = Layout::from_size_align(size, layout.align()).unwrap();

        self.holes.deallocate(ptr, layout);
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn top(&self) -> usize {
        self.bottom + self.size
    }

    // Make sure the memory in that location is free!
    pub unsafe fn extend(&mut self, by: usize) {
        let top = self.top();
        let layout = Layout::from_size_align(by, 1).unwrap();
        self.holes.deallocate(top as *mut u8, layout);
        self.size += by;
    }
}

unsafe impl Alloc for Heap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        self.allocate_first_fit(layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.deallocate(ptr, layout);
    }
}

pub struct LockedHeap(Mutex<Heap>);

impl LockedHeap {
    // Create an empty heap
    pub const fn empty() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::empty()))
    }

    // New heap with given bottom and size. Make sure the memory exists!
    pub unsafe fn new(heap_bottom: usize, heap_size: usize) -> LockedHeap {
        LockedHeap(Mutex::new(Heap {
            bottom: heap_bottom,
            size: heap_size,
            holes: HoleList::new(heap_bottom, heap_size),
        }))
    }
}

impl Deref for LockedHeap {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<'a> Alloc for &'a LockedHeap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        self.0.lock().allocate_first_fit(layout)
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.0.lock().deallocate(ptr, layout);
    }
}