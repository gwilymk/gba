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
