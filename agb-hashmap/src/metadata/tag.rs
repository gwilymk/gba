use core::fmt::Debug;

use crate::HashType;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct Tag(u8);
impl Tag {
    pub(crate) const EMPTY: Tag = Tag(0b1111_1111);
    pub(crate) const TOMBSTONE: Tag = Tag(0b1000_0000);

    pub(crate) const fn has_entry(self) -> bool {
        self.0 & 0b1000_0000 == 0
    }

    pub(crate) const fn make_full(hash: HashType) -> Self {
        // Grab the top 7 bits of the hash and use those
        let top7_bits = hash.0 >> (32 - 7);
        Tag((top7_bits & 0b0111_1111) as u8)
    }
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

#[cfg(test)]
#[allow(clippy::missing_panics_doc)]
mod test {
    use super::*;

    #[test]
    fn empty_and_tombstone_have_no_entry() {
        assert!(!Tag::TOMBSTONE.has_entry());
        assert!(!Tag::EMPTY.has_entry());
    }
}
