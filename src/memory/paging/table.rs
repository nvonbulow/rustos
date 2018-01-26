use core::ops::{Index, IndexMut};
use core::marker::PhantomData;

use memory::FrameAllocator;
use super::entry::*;
use super::PAGE_ENTRY_COUNT;

pub const P4: *mut Table<Level4> = 0o177777_777_777_777_777_0000 as *mut _;

pub struct Table<L: TableLevel> {
    entries: [PageEntry; PAGE_ENTRY_COUNT],
    level: PhantomData<L>
}

impl<L> Table<L> where L: TableLevel {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L> Table<L> where L: HeirarchialLevel {
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &*(address as *const _) })
    }

    pub fn next_table_mut(&self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &mut *(address as *mut _) })
    }

    pub fn next_table_create<A>(&mut self, index: usize, allocator: &mut A) -> &mut Table<L::NextLevel>
    where A: FrameAllocator {
        if self.next_table(index).is_none() {
            assert!(!self.entries[index].flags().contains(EntryFlags::HUGE_PAGE),
                "mapping code does not support huve pages yet");
            let frame = allocator.allocate_frame().expect("No frames available");
            self.entries[index].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            self.next_table_mut(index).unwrap().zero();
        }
        self.next_table_mut(index).unwrap()
    }

    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        // Next table address is only valid if it's present and this entry is not huge
        if entry_flags.contains(EntryFlags::PRESENT) && !entry_flags.contains(EntryFlags::HUGE_PAGE) {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | index << 12)
        }
        else {
            None
        }
    }
}

impl<L> Index<usize> for Table<L> where L: TableLevel {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &PageEntry {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L> where L: TableLevel {
    fn index_mut(&mut self, index: usize) -> &mut PageEntry {
        &mut self.entries[index]
    }
}

pub trait TableLevel {}
pub trait HeirarchialLevel : TableLevel {
    type NextLevel: TableLevel;
}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

impl HeirarchialLevel for Level4 {
    type NextLevel = Level3;
}
impl HeirarchialLevel for Level3 {
    type NextLevel = Level2;
}
impl HeirarchialLevel for Level2 {
    type NextLevel = Level1;
}
