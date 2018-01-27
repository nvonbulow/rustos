pub mod area_frame_allocator;
pub mod heap_allocator;
pub mod paging;
pub mod stack_allocator;

use multiboot2::BootInformation;

use self::paging::entry::EntryFlags;
pub use self::area_frame_allocator::AreaFrameAllocator;

pub const PAGE_SIZE: usize = 4096;

pub use self::stack_allocator::Stack;

pub struct MemoryController {
    active_table: paging::ActivePageTable,
    frame_allocator: AreaFrameAllocator,
    stack_allocator: stack_allocator::StackAllocator,
}

impl MemoryController {
    pub fn alloc_stack(&mut self, size_in_pages: usize) -> Option<Stack> {
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            ref mut stack_allocator,
        } = self;

        stack_allocator.alloc_stack(active_table, frame_allocator, size_in_pages)
    }
}

#[allow(dead_code)]
pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    }
    else if align == 0 {
        addr
    }
    else {
        panic!("`align` must be a power of two!");
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

#[allow(dead_code)]
impl Frame {
    pub fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start,
            end,
        }
    }

    fn containing_address(addr: usize) -> Frame {
        Frame {
            number: addr / PAGE_SIZE // Truncate down
        }
    }

    fn clone(&self) -> Frame {
        Frame {
            number: self.number
        }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    pub fn end_address(&self) -> usize {
        (self.number + 1) * PAGE_SIZE - 1
    }
}

pub struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        }
        else {
            None
        }
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

pub fn init(boot_info: &BootInformation) -> MemoryController {
    assert_has_not_been_called!("memory::init must only be called once!");
    let memory_map_tag = boot_info.memory_map_tag().expect("Memory Map Tag Required");
    let elf_sections_tag = boot_info.elf_sections_tag().expect("Elf Sections Tag Required!");

    let kernel_start = elf_sections_tag.sections()
        // Ignore sections that aren't allocated such as debug sections
        .filter(|s| s.is_allocated())
        .map(|s| s.addr)
        .min().unwrap();
    let kernel_end = elf_sections_tag.sections()
        .filter(|s| s.is_allocated())
        .map(|s| s.addr + s.size)
        .max().unwrap();

    kprintln!("Kernel start: {:#x}; Kernel end: {:#x}", kernel_start, kernel_end);
    kprintln!("Multiboot start: {:#x}; Multiboot end: {:#x}", boot_info.start_address(), boot_info.end_address());

    let mut frame_allocator = AreaFrameAllocator::new(
        kernel_start as usize, kernel_end as usize,
        boot_info.start_address(), boot_info.end_address(),
        memory_map_tag.memory_areas());
    let mut active_table = self::paging::remap_the_kernel(&mut frame_allocator, boot_info);

    use self::paging::Page;
    use {HEAP_START, HEAP_SIZE};

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    active_table.map_range(
        Page::range_inclusive(heap_start_page, heap_end_page),
        EntryFlags::WRITABLE, &mut frame_allocator
    );

    let stack_allocator = {
        let stack_alloc_start = heap_end_page + 1;
        let stack_alloc_end = stack_alloc_start + 100;
        let stack_alloc_range = Page::range_inclusive(stack_alloc_start, stack_alloc_end);
        stack_allocator::StackAllocator::new(stack_alloc_range)
    };

    MemoryController {
        active_table,
        frame_allocator,
        stack_allocator,
    }
}
