
use super::paging::{self, PageIter, Page, ActivePageTable};
use super::paging::entry::EntryFlags;
use super::{PAGE_SIZE, FrameAllocator};

pub struct StackAllocator {
    range: PageIter,
}

impl StackAllocator {
    pub fn new(range: PageIter) -> StackAllocator {
        StackAllocator {
            range
        }
    }

    pub fn alloc_stack<FA: FrameAllocator>(&mut self,
                           active_table: &mut ActivePageTable,
                           frame_allocator: &mut FA,
                           size_in_pages: usize) -> Option<Stack> {
        if size_in_pages == 0 {
            return None;
        }

        let mut range = self.range.clone();

        let guard_page = range.next();
        let stack_start = range.next();
        let stack_end = if size_in_pages == 1 {
            stack_start
        }
        else {
            // choose the (size_in_pages)-2 because we already allocated the start page and index
            // starts at zero
            range.nth(size_in_pages - 2)
        };

        match (guard_page, stack_start, stack_end) {
            (Some(_), Some(start), Some(end)) => {
                // Was successful so write it back
                self.range = range;

                active_table.map_range(Page::range_inclusive(start, end), EntryFlags::WRITABLE, frame_allocator);

                let top_of_stack = end.start_address() + PAGE_SIZE;
                Some(Stack::new(top_of_stack, start.start_address()))
            },
            _ => None, // Not enough pages
        }
    }
}

#[derive(Debug)]
pub struct Stack {
    top: usize,
    bottom: usize,
}

impl Stack {
    fn new(top: usize, bottom: usize) -> Stack {
        assert!(top > bottom);
        Stack {
            top,
            bottom,
        }
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }
}