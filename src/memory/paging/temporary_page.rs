use super::{Page, ActivePageTable, VirtualAddress};
use super::{Table, Level1};
use memory::{Frame, FrameAllocator};

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator
}

impl TemporaryPage {

    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage where A: FrameAllocator {
        TemporaryPage {
            page,
            allocator: TinyAllocator::new(allocator)
        }
    }

    // maps the temporary page to the given frame in the active table
    // returns start address of the temporary page
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> VirtualAddress {
        use super::entry::EntryFlags;

        assert!(active_table.translate_page(self.page).is_none(),
            "temporary page is already mapped");
        active_table.map_to(self.page, frame, EntryFlags::WRITABLE, &mut self.allocator);
        self.page.start_address()
    }

    // Maps the temporary page to the given page table frame in the active table
    // Returns a reference to the now mapped table. It's a level 1 table because it's not recursively mapped
    pub fn map_table_frame(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }

    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page, &mut self.allocator);
    }
}

struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    fn new<A>(allocator: &mut A) -> TinyAllocator where A: FrameAllocator {
        let mut f = || allocator.allocate_frame();
        let frames = [f(), f(), f()];
        TinyAllocator(frames)
    }
}

impl FrameAllocator for TinyAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }
        panic!("Tiny allocator can only hold three frames!");
    }
}
