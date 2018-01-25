use multiboot2::ElfSection;

use memory::Frame;

pub struct PageEntry(u64);

impl PageEntry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> PageEntryFlags {
        PageEntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(PageEntryFlags::PRESENT) {
            Some(Frame::containing_address(
                self.0 as usize & 0x000fffff_fffff000
            ))
        }
        else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: PageEntryFlags) {
        assert_eq!(frame.start_address() & !0x000fffff_fffff000, 0);
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

bitflags! {
    pub struct PageEntryFlags: u64 {
        const PRESENT         = 1 << 0;
        const WRITABLE        = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH   = 1 << 3;
        const NO_CACHE        = 1 << 4;
        const ACCESSED        = 1 << 5;
        const DIRTY           = 1 << 6;
        const HUGE_PAGE       = 1 << 7;
        const GLOBAL          = 1 << 8;
        // Bits 9-11 are available for our use
        // Bits 12-51 is a page aligned address of the frame/next page table
        // Bits 52-62 are available for our use
        const NO_EXECUTE      = 1 << 63;
    }
}

impl PageEntryFlags {
    pub fn from_elf_section_flags(section: &ElfSection) -> PageEntryFlags {
        use multiboot2::{ELF_SECTION_ALLOCATED, ELF_SECTION_WRITABLE, ELF_SECTION_EXECUTABLE};

        let mut flags = PageEntryFlags::empty();

        if section.flags().contains(ELF_SECTION_ALLOCATED) {
            // the section is loaded to memory
            flags = flags | PageEntryFlags::PRESENT;
        }
        if section.flags().contains(ELF_SECTION_WRITABLE) {
            flags = flags | PageEntryFlags::WRITABLE;
        }
        if !section.flags().contains(ELF_SECTION_EXECUTABLE) {
            flags = flags | PageEntryFlags::NO_EXECUTE;
        }

        flags
    }
}
