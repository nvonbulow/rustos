use super::{VirtualAddress, PhysicalAddress, Page, PageIter, PAGE_ENTRY_COUNT};
use super::entry::*;
use super::table::{self, Table, Level4, Level1};
use memory::{PAGE_SIZE, Frame, FrameIter, FrameAllocator};
use core::ptr::Unique;

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    pub unsafe fn new() -> Mapper {
        Mapper {
            p4: Unique::new_unchecked(table::P4),
        }
    }

    pub fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn map<A>(&mut self, page: Page, flags: PageEntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        let frame = allocator.allocate_frame().expect("Out of memory");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn map_range<A>(&mut self, pages: PageIter, flags: PageEntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        for page in pages {
            self.map(page, flags, allocator);
        }
    }

    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: PageEntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        let mut p3 = self.p4_mut().next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PageEntryFlags::PRESENT);
    }

    pub fn identity_map_range<A>(&mut self, frames: FrameIter, flags: PageEntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        for frame in frames {
            &mut self.identity_map(frame, flags, allocator);
        }
    }

    pub fn identity_map<A>(&mut self, frame: Frame, flags: PageEntryFlags, allocator: &mut A)
        where A: FrameAllocator {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
        where A: FrameAllocator {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.p4_mut()
                     .next_table_mut(page.p4_index())
                     .and_then(|p3| p3.next_table_mut(page.p3_index()))
                     .and_then(|p2| p2.next_table_mut(page.p2_index()))
                     .expect("Huge pages are not supported yet");

        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        use x86_64::instructions::tlb;
        use x86_64::VirtualAddress;
        tlb::flush(VirtualAddress(page.start_address()));

        // allocator.deallocate_frame(frame);
    }

    pub fn translate(&self, vaddr: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = vaddr % PAGE_SIZE;
        self.translate_page(Page::containing_address(vaddr))
            .map(|frame| frame.number * PAGE_SIZE + offset)
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = unsafe { &*table::P4 }.next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // Check for 1GiB page
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(PageEntryFlags::HUGE_PAGE) {
                        assert_eq!(start_frame.number % (PAGE_ENTRY_COUNT * PAGE_ENTRY_COUNT), 0);
                        return Some(Frame {
                            number: start_frame.number + page.p2_index() * PAGE_ENTRY_COUNT + page.p1_index()
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // Check for 2MiB page
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(PageEntryFlags::HUGE_PAGE) {
                            assert_eq!(start_frame.number % PAGE_ENTRY_COUNT, 0);
                            return Some(Frame {
                                number: start_frame.number + page.p1_index()
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
          .and_then(|p2| p2.next_table(page.p2_index()))
          .and_then(|p1| p1[page.p1_index()].pointed_frame())
          .or_else(huge_page)
    }
}