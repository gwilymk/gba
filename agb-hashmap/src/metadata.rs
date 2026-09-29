use crate::{Allocator, HashType, metadata::tag::Tag, my_vec::MyVec};
use core::fmt::Debug;

mod tag;

#[derive(Clone)]
pub(crate) struct Metadata<A: Allocator> {
    control: MyVec<u32, A>,
    slots: MyVec<u16, A>,

    // 32 - log2(slots / 4)
    group_shift: u8,
    // EMPTY slots we can use before rehash or grow
    growth_left: u32,
    // 1 << size_shift is the total size (minimum 3 so if less than that, this is empty)
    size_shift: u8,
}

impl<A: Allocator + Copy> Metadata<A> {
    pub(crate) const fn new_in(allocator: A) -> Self {
        Self {
            control: MyVec::new_in(allocator),
            slots: MyVec::new_in(allocator),

            group_shift: 0,
            growth_left: 0,
            size_shift: 0,
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

        let shift = u8::try_from(size.trailing_zeros()).expect("Trailing zeros is at most 32");

        let mut control = MyVec::with_capacity_in(size, allocator);
        let mut slots = MyVec::with_capacity_in(size, allocator);

        control.resize(size / 4, 0xffff_ffff);
        slots.resize(size, 0);

        let (growth_left, group_shift) = growth_left_and_group_shift(shift);

        Self {
            control,
            slots,

            group_shift,
            growth_left,
            size_shift: shift,
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity_u32() as usize
    }

    fn capacity_u32(&self) -> u32 {
        if self.size_shift < 3 {
            return 0;
        }

        let size = 1 << self.size_shift;
        size - size / 4
    }

    pub(crate) fn clear(&mut self) {
        self.control.fill(0xffff_ffff);

        self.growth_left = self.capacity_u32();
    }

    /// Note that f and the return value refer to the location in the metadata, not the location in the hashmap.
    /// Call `unwrap_slot()` to turn the returned value and the values here into hashmap node entries.
    pub(crate) fn find_entry_matching(
        &self,
        hash: HashType,
        f: impl Fn(SlotId) -> bool,
    ) -> Option<SlotId> {
        if self.control.is_empty() {
            // empty hashmap
            return None;
        }

        let new_tag = Tag::make_full(hash, self.group_shift);

        let mut g = (hash.0 >> self.group_shift) as usize;
        let mut stride = 0;
        let group_mask = (1 << (self.size_shift - 2)) - 1;

        loop {
            // check the whole group at once
            let ctrl = self.control[g];
            let mut m = tag::match_tag(ctrl, new_tag);
            while m != 0 {
                let slot = SlotId(g * 4 + tag::lowest_non_zero_byte(m));
                if f(slot) {
                    return Some(slot);
                }

                m &= m - 1;
            }

            if tag::match_empty(ctrl) != 0 {
                return None;
            }

            stride += 1;
            g = (g + stride) & group_mask;
        }
    }

    pub(crate) fn unwrap_slot(&self, slot_id: SlotId) -> usize {
        self.slots[slot_id.0] as usize
    }

    pub(crate) fn check_size(&self, total_entries: usize) -> Result<(), Resize> {
        if total_entries >= self.capacity() {
            Err(Resize::Grow)
        } else if self.growth_left == 0 {
            Err(Resize::RehashInPlace)
        } else {
            Ok(())
        }
    }

    pub(crate) fn grow(&mut self, hashes: impl Iterator<Item = HashType>) {
        if self.size_shift < 3 {
            self.size_shift = 3;
        } else {
            self.size_shift += 1;
        }
        let new_size = 1 << self.size_shift;
        self.group_shift = growth_left_and_group_shift(self.size_shift).1;

        self.control.resize(new_size / 4, 0xffff_ffff);
        self.slots.resize(new_size, 0);

        self.rehash_in_place(hashes);
    }

    pub(crate) fn rehash_in_place(&mut self, hashes: impl Iterator<Item = HashType>) {
        self.control.fill(0xffff_ffff);
        self.growth_left = self.capacity_u32();

        for (i, hash) in hashes.enumerate() {
            self.insert_new(hash, i);
        }
    }

    pub(crate) fn insert_new(&mut self, hash: HashType, hashmap_table_index: usize) {
        debug_assert!(
            self.growth_left > 0,
            "Should have space to put the entry, capacity: {}",
            self.capacity()
        );

        let new_tag = Tag::make_full(hash, self.group_shift);

        let mut g = (hash.0 >> self.group_shift) as usize;
        let mut stride = 0;
        let group_mask = (1 << (self.size_shift - 2)) - 1;

        loop {
            // check the whole group at once
            let mut ctrl = self.control[g];
            let m = tag::match_empty_or_deleted(ctrl);
            if m != 0 {
                let lowest_byte = tag::lowest_non_zero_byte(m);
                let slot = g * 4 + lowest_byte;

                let shift_amount = lowest_byte * 8;

                // grab the current slot value
                let current_slot_value = (ctrl >> shift_amount) & 0xff;
                let current_tag = Tag::from_raw(current_slot_value as u8);
                if current_tag == Tag::EMPTY {
                    self.growth_left -= 1;
                }

                ctrl &= !(0xff << shift_amount);
                ctrl |= new_tag.raw() << shift_amount;

                self.control[g] = ctrl;

                self.set_slot_id(SlotId(slot), hashmap_table_index);

                return;
            }

            stride += 1;
            g = (g + stride) & group_mask;
        }
    }

    pub(crate) fn tombstone(&mut self, location: SlotId) {
        let (location_idx, location_offset) = (location.0 / 4, (location.0 & 3) * 8);
        self.control[location_idx] &= !(0xff << location_offset);
        self.control[location_idx] |= Tag::TOMBSTONE.raw() << location_offset;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub(crate) fn set_slot_id(&mut self, metadata_slot: SlotId, data_slot: usize) {
        self.slots[metadata_slot.0] = data_slot as u16;
    }

    /// # Panics
    ///
    /// If invariants aren't held
    #[cfg(test)]
    pub(crate) fn assert_invariants(&self) {}
}

fn growth_left_and_group_shift(size_shift: u8) -> (u32, u8) {
    if size_shift == 0 {
        return (0, 0);
    }

    let size = 1 << size_shift;
    (size - size / 4, 32 - size_shift + 2)
}

pub(crate) enum Resize {
    Grow,
    RehashInPlace,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SlotId(usize);

impl<A: Allocator> Debug for Metadata<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Metadata(group_shift: {}, growth_left: {}, size_shift: {}, data: ",
            self.group_shift, self.growth_left, self.size_shift
        )?;
        for (ctrl, slot) in self.control.iter().zip(self.slots.iter()) {
            write!(f, "{ctrl:?} - {slot}, ")?;
        }

        write!(f, ")")
    }
}
