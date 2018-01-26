use core::ops::{Add, Deref, DerefMut};
use core::ptr::Unique;
use multiboot2::BootInformation;

use memory::PAGE_SIZE;
use super::{Frame, FrameAllocator};
use self::entry::EntryFlags;
use self::mapper::Mapper;
use self::table::{Table, Level1, Level4};
use self::temporary_page::TemporaryPage;

pub mod entry;
mod mapper;
pub mod table;
mod temporary_page;

// Number of entries in a page table
const PAGE_ENTRY_COUNT: usize = 512;

type VirtualAddress = usize;
type PhysicalAddress = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        // addresses must have sign extension
        assert!(address < 0x0000_8000_0000_0000 ||
            address >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}", address);
        Page { number: address / PAGE_SIZE }
    }

    pub fn start_address(&self) -> VirtualAddress {
        self.number * PAGE_SIZE
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start,
            end,
        }
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }

    fn p1_index (&self) -> usize {
        (self.number >> 0) & 0o777
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Self::Output {
        Page {
            number: self.number + rhs
        }
    }
}

#[derive(Copy, Clone)]
pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        }
        else {
            None
        }
    }
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Self::Target {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn with<F: FnOnce(&mut Mapper)>(&mut self,
                   table: &mut InactivePageTable,
                   temporary_page: &mut temporary_page::TemporaryPage,
                   f: F) {
        use x86_64::instructions::tlb;
        use x86_64::registers::control_regs;

        {
            let backup = Frame::containing_address(unsafe { control_regs::cr3().0 } as usize);
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // overwrite recursive mapping
            self.p4_mut()[511].set(table.p4_frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();

            // Execute f in the context of the inactive table
            f(self);

            // restore recursive mapping to original p4 table
            p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }
        temporary_page.unmap(self);
    }

    // Returns the old page table
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64::PhysicalAddress;
        use x86_64::registers::control_regs;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(control_regs::cr3().0 as usize),
        };

        unsafe {
            control_regs::cr3_write(PhysicalAddress(new_table.p4_frame.start_address() as u64));
        }
        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable, temporary_page: &mut TemporaryPage) -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            table.zero();
            // Set up recursive mapping
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }
        temporary_page.unmap(active_table);
        InactivePageTable {
            p4_frame: frame
        }
    }
}

pub fn remap_the_kernel<A: FrameAllocator>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable {
    let mut temporary_page = TemporaryPage::new(Page { number: 0xcafebabe },
        allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("No more frames!");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections_tag().expect("memory map tag required");

        for section in elf_sections_tag.sections() {
            // identity map all pages of section (or move elsewhere but that would require PIC)
            if !section.is_allocated() {
                // section is not loaded
                continue;
            }
            assert_eq!(section.start_address() % PAGE_SIZE, 0, "sections need to be page aligned!");

            kprintln!("mapping section at addr: {:#x}, size: {:#x}", section.addr, section.size);

            let flags = EntryFlags::from_elf_section_flags(section);

            let start_frame = Frame::containing_address(section.start_address());
            let end_frame = Frame::containing_address(section.end_address() - 1);
            mapper.identity_map_range(
                Frame::range_inclusive(start_frame, end_frame),
                flags, allocator);
        }
        mapper.identity_map_range(
            Frame::range_inclusive(
                Frame::containing_address(boot_info.start_address()),
                Frame::containing_address(boot_info.end_address())),
            EntryFlags::PRESENT,
            allocator
        );
        mapper.identity_map(Frame::containing_address(0xb8000),
                            EntryFlags::WRITABLE, allocator);
    });

    let old_table = active_table.switch(new_table);

    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    // Create a guard page. The stack is right above the old page table so use p1, p2, and p3
    // as extra space and a page fault will occur on stack overflow instead of silent corruption
    active_table.unmap(old_p4_page, allocator);
    kprintln!("Guard Page at 0x{:x}", old_p4_page.start_address());
    active_table
}
