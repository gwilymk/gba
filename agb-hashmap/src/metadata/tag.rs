use core::fmt::Debug;

use crate::HashType;

const LSB: u32 = 0x0101_0101;
const MSB: u32 = 0x8080_8080;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct Tag(u8);
impl Tag {
    pub(crate) const EMPTY: Tag = Tag(0b1111_1111);
    pub(crate) const TOMBSTONE: Tag = Tag(0b1000_0000);

    pub(crate) const fn make_full(hash: HashType, group_shift: u8) -> Self {
        // Grab the next 7 bits after the group shift
        let top7_bits = hash.0 >> (group_shift - 7);
        Tag((top7_bits & 0b0111_1111) as u8)
    }

    pub(crate) const fn from_raw(value: u8) -> Self {
        Self(value)
    }

    pub(crate) const fn raw(self) -> u32 {
        self.0 as u32
    }
}

pub(crate) fn match_tag(group: u32, tag: Tag) -> u32 {
    let x = group ^ (LSB * tag.0 as u32);
    x.wrapping_sub(LSB) & !x & MSB
}

pub(crate) fn match_empty(group: u32) -> u32 {
    group & (group << 1) & MSB
}

pub(crate) fn match_empty_or_deleted(group: u32) -> u32 {
    group & MSB
}

pub(crate) fn lowest_non_zero_byte(m: u32) -> usize {
    let v = m.isolate_lowest_one() >> 7;
    (v.wrapping_mul(0x0001_0203) >> 24) as usize
}

impl Debug for Tag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self == &Self::EMPTY {
            write!(f, "Tag(EMPTY)")
        } else if self == &Self::TOMBSTONE {
            write!(f, "Tag(TOMBSTONE)")
        } else {
            write!(f, "Tag(Full({}))", self.0 & 0b0111_1111)
        }
    }
}
