use crate::{Allocator, HashType, metadata::tag::Tag, my_vec::MyVec};

mod tag;

#[derive(Clone)]
pub(crate) struct Metadata<A: Allocator> {
    control: MyVec<Tag, A>,
    slots: MyVec<u16, A>,

    filled_slots: usize,
}

impl<A: Allocator + Copy> Metadata<A> {
    pub(crate) const fn new_in(allocator: A) -> Self {
        Self {
            control: MyVec::new_in(allocator),
            slots: MyVec::new_in(allocator),

            filled_slots: 0,
        }
    }

    pub(crate) fn allocator(&self) -> &A {
        self.control.allocator()
    }

    /// # Panics
    ///
    /// If size isn't a power of 2 or greater than or equal to `u16::MAX`
    pub(crate) fn with_size(mut size: usize, allocator: A) -> Metadata<A> {
        if size <= 4 {
            size = 8;
        }

        assert!(size.is_power_of_two());
        assert!(size < u16::MAX as usize);

        let mut control = MyVec::with_capacity_in(size, allocator);
        let mut slots = MyVec::with_capacity_in(size, allocator);

        control.resize(size, Tag::EMPTY);
        slots.resize(size, 0);

        Self {
            control,
            slots,
            filled_slots: 0,
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        let size = self.control.len();
        size - size / 4
    }

    pub(crate) fn clear(&mut self) {
        self.control.fill(Tag::EMPTY);
        self.filled_slots = 0;
    }

    /// Note that f and the return value refer to the location in the metadata, not the location in the hashmap.
    /// Call `unwrap_slot()` to turn the returned value and the values here into hashmap node entries.
    pub(crate) fn find_entry_matching(
        &self,
        hash: HashType,
        f: impl Fn(SlotId) -> bool,
    ) -> Option<SlotId> {
        if self.filled_slots == 0 {
            // empty hashmap
            return None;
        }

        let search_tag = Tag::make_full(hash);

        let mut index = self.search_start_point(hash);
        while self.control[index.0] != Tag::EMPTY {
            if self.control[index.0] == search_tag && f(index) {
                return Some(index);
            }

            index = SlotId((index.0 + 1) & (self.control.len() - 1));
        }

        None
    }

    pub(crate) fn unwrap_slot(&self, slot_id: SlotId) -> usize {
        self.slots[slot_id.0] as usize
    }

    pub(crate) fn check_size(&self, data_entries: usize) -> Result<(), Resize> {
        if data_entries >= self.capacity() {
            Err(Resize::Grow)
        } else if self.filled_slots - data_entries > self.capacity() / 2 {
            Err(Resize::RehashInPlace)
        } else {
            Ok(())
        }
    }

    pub(crate) fn grow(&mut self, hashes: impl Iterator<Item = HashType>) {
        self.filled_slots = 0;
        let new_size = (self.control.len() * 2).max(8);

        self.control.resize(new_size, Tag::EMPTY);
        self.slots.resize(new_size, 0);

        self.rehash_in_place(hashes);
    }

    pub(crate) fn rehash_in_place(&mut self, hashes: impl Iterator<Item = HashType>) {
        extern crate std;
        std::dbg!("Rehashing");

        self.control.fill(Tag::EMPTY);
        self.slots.fill(0);

        for (i, hash) in hashes.enumerate() {
            self.insert_new(hash, i);
        }
    }

    pub(crate) fn insert_new(&mut self, hash: HashType, hashmap_table_index: usize) {
        let new_tag = Tag::make_full(hash);

        let mut index = self.search_start_point(hash);
        loop {
            if !self.control[index.0].has_entry() {
                let old_tag = core::mem::replace(&mut self.control[index.0], new_tag);
                self.set_slot_id(index, hashmap_table_index);

                // tombstones don't count towards the filled slot count
                if old_tag == Tag::EMPTY {
                    self.filled_slots += 1;
                }

                return;
            }

            index = SlotId((index.0 + 1) & (self.control.len() - 1));
        }
    }

    fn search_start_point(&self, hash: HashType) -> SlotId {
        SlotId(hash.fast_mod(self.control.len()))
    }

    pub(crate) fn tombstone(&mut self, location: SlotId) {
        self.control[location.0] = Tag::TOMBSTONE;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub(crate) fn set_slot_id(&mut self, metadata_slot: SlotId, data_slot: usize) {
        self.slots[metadata_slot.0] = data_slot as u16;
    }
}

pub(crate) enum Resize {
    Grow,
    RehashInPlace,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotId(usize);
