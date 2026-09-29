//! A hash map implementation optimised for the Game Boy Advance.
//!
//! The main structs are [`HashMap`] and [`HashSet`].
//!
//! A lot of the documentation for this module was copied straight out of the rust
//! standard library. The implementation however is not.
#![no_std]
#![cfg_attr(feature = "allocator_api", feature(allocator_ext))]
#![deny(clippy::all)]
#![deny(clippy::must_use_candidate)]
#![deny(missing_docs)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::semicolon_if_nothing_returned)]
#![deny(clippy::map_unwrap_or)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::redundant_closure_for_method_calls)]
#![deny(clippy::cloned_instead_of_copied)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(unreachable_pub)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::manual_assert)]
#![deny(clippy::default_trait_access)]
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::doc_markdown)]
#![deny(clippy::return_self_not_must_use)]
#![deny(clippy::cast_possible_truncation)]

extern crate alloc;

pub(crate) use allocate::{Allocator, Global};
use rustc_hash::FxHasher;

#[cfg(not(feature = "allocator_api"))]
mod allocate {
    pub trait Allocator {}

    #[derive(Copy, Clone)]
    pub struct Global;

    impl Allocator for Global {}
}

#[cfg(feature = "allocator_api")]
mod allocate {
    pub(crate) use alloc::alloc::Global;
    pub(crate) use core::alloc::Allocator;
}

#[cfg(feature = "serde")]
mod serde;

use core::{
    borrow::Borrow,
    fmt::Debug,
    hash::{Hash, Hasher},
    num::Wrapping,
    ops::Index,
};

mod hash_set;
mod metadata;

mod my_vec;
use my_vec::MyVec;

pub use hash_set::HashSet;

/// It is required that the keys implement the [`Eq`] and [`Hash`] traits, although this
/// can be frequently achieved by using `#[derive(PartialEq, Eq, Hash)]`. If you
/// implement these yourself, it is important that the following property holds:
///
/// `k1 == k2 => hash(k1) == hash(k2)`
///
/// It is a logic error for the key to be modified in such a way that the key's hash, as
/// determined by the [`Hash`] trait, or its equality as determined by the [`Eq`] trait,
/// changes while it is in the map. The behaviour for such a logic error is not specified,
/// but will not result in undefined behaviour. This could include panics, incorrect results,
/// aborts, memory leaks and non-termination.
///
/// The API surface provided is incredibly similar to the
/// [`std::collections::HashMap`](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
/// implementation with fewer guarantees, and better optimised for the `GameBoy Advance`.
///
/// [`Eq`]: https://doc.rust-lang.org/core/cmp/trait.Eq.html
/// [`Hash`]: https://doc.rust-lang.org/core/hash/trait.Hash.html
///
/// # Example
/// ```
/// use agb_hashmap::HashMap;
///
/// // Type inference lets you omit the type signature (which would be HashMap<String, String> in this example)
/// let mut game_reviews = HashMap::new();
///
/// // Review some games
/// game_reviews.insert(
///     "Pokemon Emerald".to_string(),
///     "Best post-game battle experience of any generation.".to_string(),
/// );
/// game_reviews.insert(
///     "Golden Sun".to_string(),
///     "Some of the best music on the console".to_string(),
/// );
/// game_reviews.insert(
///     "Super Dodge Ball Advance".to_string(),
///     "Really great launch title".to_string(),
/// );
///
/// // Check for a specific entry
/// if !game_reviews.contains_key("Legend of Zelda: The Minish Cap") {
///     println!("We've got {} reviews, but The Minish Cap ain't one", game_reviews.len());
/// }
///
/// // Iterate over everything
/// for (game, review) in &game_reviews {
///     println!("{game}: \"{review}\"");
/// }
/// ```
#[derive(Clone)]
pub struct HashMap<K, V, DATA: Allocator = Global, METADATA: Allocator = Global> {
    metadata: Metadata<METADATA>,
    data: MyVec<HashMapNode<K, V>, DATA>,
}

#[derive(Clone)]
struct HashMapNode<K, V> {
    hash: HashType,
    key: K,
    value: V,
}

impl<K, V> HashMap<K, V> {
    /// Creates a `HashMap`
    #[must_use]
    pub const fn new() -> Self {
        Self::new_in(Global, Global)
    }

    /// Creates an empty `HashMap` which can hold at least `capacity` elements before resizing.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity, Global, Global)
    }
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> HashMap<K, V, DATA, METADATA> {
    #[must_use]
    /// Creates an empty `HashMap` with specified internal size
    pub fn with_size_in(size: usize, data_alloc: DATA, metadata_alloc: METADATA) -> Self {
        Self {
            metadata: Metadata::with_size(size, metadata_alloc),
            data: MyVec::with_capacity_in(size, data_alloc),
        }
    }

    #[must_use]
    /// Creates a `HashMap` with a specified allocator for data and metadata
    pub const fn new_in(data_alloc: DATA, metadata_alloc: METADATA) -> Self {
        Self {
            metadata: Metadata::new_in(metadata_alloc),
            data: MyVec::new_in(data_alloc),
        }
    }

    /// Returns a reference to the underlying allocator for data
    pub fn data_allocator(&self) -> &DATA {
        self.data.allocator()
    }

    /// Returns a reference to the underlying allocator for metadata
    pub fn metadata_allocator(&self) -> &METADATA {
        self.metadata.allocator()
    }

    /// Creates an empty `HashMap` which can hold at least `capacity` elements before resizing. The actual
    /// internal size may be larger as it must be a power of 2
    #[must_use]
    pub fn with_capacity_in(capacity: usize, data_alloc: DATA, metadata_alloc: METADATA) -> Self {
        let final_size = if capacity.is_power_of_two() {
            capacity
        } else {
            capacity.next_power_of_two()
        };

        Self::with_size_in(final_size, data_alloc, metadata_alloc)
    }

    /// Returns the number of elements in the map
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of elements the map can hold before resizing
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.metadata.capacity()
    }

    /// An iterator visiting all keys in an arbitrary order
    pub fn keys(&self) -> impl Iterator<Item = &'_ K> {
        self.iter().map(|(k, _)| k)
    }

    /// An iterator visiting all values in an arbitrary order
    pub fn values(&self) -> impl Iterator<Item = &'_ V> {
        self.iter().map(|(_, v)| v)
    }

    /// An iterator visiting all values in an arbitrary order allowing for mutation
    pub fn values_mut(&mut self) -> impl Iterator<Item = &'_ mut V> {
        self.iter_mut().map(|(_, v)| v)
    }

    /// Removes all elements from the map
    pub fn clear(&mut self) {
        self.data.clear();
        self.metadata.clear();
    }

    /// An iterator visiting all key-value pairs in an arbitrary order
    pub fn iter(&self) -> impl Iterator<Item = (&'_ K, &'_ V)> {
        self.data.iter().map(|node| (&node.key, &node.value))
    }

    /// An iterator visiting all key-value pairs in an arbitrary order, with mutable references to the values
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&'_ K, &'_ mut V)> {
        self.data
            .iter_mut()
            .map(|node| (&node.key, &mut node.value))
    }

    /// Retains only the elements specified by the predicate `f`.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        let len_before_retain = self.data.len();
        self.data.retain_mut(|kv| f(&kv.key, &mut kv.value));
        let len_after_retain = self.data.len();

        if len_before_retain != len_after_retain {
            self.metadata
                .rehash_in_place(self.data.iter().map(|kv| kv.hash));
        }
    }

    /// Returns `true` if the map contains no elements
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Panics
    ///
    /// If the location doesn't actually exist
    fn remove_from_internal_location(&mut self, location: SlotId) -> HashMapNode<K, V> {
        self.metadata.tombstone(location);
        let location = self.metadata.unwrap_slot(location);

        let old_value = self.data.swap_remove(location);

        if location != self.data.len() {
            let new_entry_at_location = &self.data[location];
            let new_entry_slot_id = self
                .metadata
                .find_entry_matching(new_entry_at_location.hash, |slot| {
                    self.metadata.unwrap_slot(slot) == self.data.len()
                })
                .expect("We just moved this, so it should have existed");

            self.metadata.set_slot_id(new_entry_slot_id, location);
        }

        old_value
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> HashMap<K, V, DATA, METADATA>
where
    K: Eq + Hash,
{
    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated and the old value
    /// is returned. The key is not updated, which matters for types that can be `==`
    /// without being identical.
    #[doc(alias("set", "add"))]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = hash(&key);

        if let Some(current_index) = self.metadata.find_entry_matching(hash, |slot| {
            let node_id = self.metadata.unwrap_slot(slot);
            self.data[node_id].key == key
        }) {
            let node_id = self.metadata.unwrap_slot(current_index);
            return Some(core::mem::replace(&mut self.data[node_id].value, value));
        }

        self.insert_new(hash, key, value);

        None
    }

    fn insert_new(&mut self, hash: HashType, key: K, value: V) -> &mut V {
        let idx = self.data.len();
        self.data.push(HashMapNode { hash, key, value });

        match self.metadata.check_size(idx + 1) {
            Err(Resize::Grow) => {
                self.metadata.grow(self.data.iter().map(|node| node.hash));
            }
            Err(Resize::RehashInPlace) => {
                self.metadata
                    .rehash_in_place(self.data.iter().map(|node| node.hash));
            }
            Ok(_) => {
                self.metadata.insert_new(hash, idx);
            }
        }

        &mut self.data[idx].value
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash(k);

        self.metadata
            .find_entry_matching(hash, |slot| {
                let node_id = self.metadata.unwrap_slot(slot);
                self.data[node_id].key.borrow() == k
            })
            .is_some()
    }

    /// Returns the key-value pair corresponding to the supplied key
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash(key);

        let location = self.metadata.find_entry_matching(hash, |slot| {
            let node_id = self.metadata.unwrap_slot(slot);
            self.data[node_id].key.borrow() == key
        })?;

        let data = &self.data[self.metadata.unwrap_slot(location)];

        Some((&data.key, &data.value))
    }

    /// Returns a reference to the value corresponding to the key. Returns [`None`] if there is
    /// no element in the map with the given key.
    ///
    /// # Example
    /// ```
    /// use agb_hashmap::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert("a".to_string(), "A");
    /// assert_eq!(map.get("a"), Some(&"A"));
    /// assert_eq!(map.get("b"), None);
    /// ```
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }

    /// Returns a mutable reference to the value corresponding to the key. Return [`None`] if
    /// there is no element in the map with the given key.
    ///
    /// # Example
    /// ```
    /// use agb_hashmap::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert("a".to_string(), "A");
    ///
    /// if let Some(x) = map.get_mut("a") {
    ///     *x = "b";
    /// }
    ///
    /// assert_eq!(map["a"], "b");
    /// ```
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash(key);

        let location = self.metadata.find_entry_matching(hash, |slot| {
            let node_id = self.metadata.unwrap_slot(slot);
            self.data[node_id].key.borrow() == key
        })?;

        let data = &mut self.data[self.metadata.unwrap_slot(location)];

        Some(&mut data.value)
    }

    /// Removes the given key from the map. Returns the current value if it existed, or [`None`]
    /// if it did not.
    ///
    /// # Example
    /// ```
    /// use agb_hashmap::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash(key);

        let location = self.metadata.find_entry_matching(hash, |slot| {
            let node_id = self.metadata.unwrap_slot(slot);
            self.data[node_id].key.borrow() == key
        })?;

        Some(self.remove_from_internal_location(location).value)
    }
}

#[expect(clippy::cast_possible_truncation)]
fn hash<Q>(key: &Q) -> HashType
where
    Q: Hash + ?Sized,
{
    let mut hasher = FxHasher::default();
    key.hash(&mut hasher);

    HashType::bit_mix(hasher.finish() as u32)
}

/// An iterator over entries of a [`HashMap`]
///
/// This struct is created using the `into_iter()` method on [`HashMap`]. See its
/// documentation for more.
pub struct Iter<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator> {
    map: &'a HashMap<K, V, DATA, METADATA>,
    at: usize,
}

impl<'a, K, V, DATA: Allocator, METADATA: Allocator + Copy> Iterator
    for Iter<'a, K, V, DATA, METADATA>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.map.data.len() {
            return None;
        }

        let entry = &self.map.data[self.at];
        self.at += 1;
        Some((&entry.key, &entry.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.map.len() - self.at, Some(self.map.len() - self.at))
    }
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> ExactSizeIterator
    for Iter<'_, K, V, DATA, METADATA>
{
}

impl<'a, K, V, DATA: Allocator, METADATA: Allocator + Copy> IntoIterator
    for &'a HashMap<K, V, DATA, METADATA>
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, DATA, METADATA>;

    fn into_iter(self) -> Self::IntoIter {
        Iter { map: self, at: 0 }
    }
}

/// An iterator over entries of a [`HashMap`]
///
/// This struct is created using the `into_iter()` method on [`HashMap`] as part of its implementation
/// of the `IntoIterator` trait.
pub struct IterOwned<K, V, DATA: Allocator, METADATA: Allocator> {
    map: HashMap<K, V, DATA, METADATA>,
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> Iterator
    for IterOwned<K, V, DATA, METADATA>
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.map.data.pop()?;
        Some((value.key, value.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.map.len(), Some(self.map.len()))
    }
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> ExactSizeIterator
    for IterOwned<K, V, DATA, METADATA>
{
}

/// An iterator over entries of a [`HashMap`]
///
/// This struct is created using the `into_iter()` method on [`HashMap`] as part of its implementation
/// of the `IntoIterator` trait.
impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> IntoIterator
    for HashMap<K, V, DATA, METADATA>
{
    type Item = (K, V);
    type IntoIter = IterOwned<K, V, DATA, METADATA>;

    fn into_iter(self) -> Self::IntoIter {
        IterOwned { map: self }
    }
}

mod entries {
    use core::hash::Hash;

    use super::{Allocator, HashMap, HashType, SlotId};

    /// A view into an occupied entry in a `HashMap`. This is part of the [`crate::Entry`] enum.
    pub struct OccupiedEntry<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator> {
        key: K,
        map: &'a mut HashMap<K, V, DATA, METADATA>,
        location: SlotId,
    }

    impl<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator + Copy>
        OccupiedEntry<'a, K, V, DATA, METADATA>
    {
        pub(crate) fn new(
            key: K,
            map: &'a mut HashMap<K, V, DATA, METADATA>,
            location: SlotId,
        ) -> Self {
            Self { key, map, location }
        }

        /// Gets a reference to the key in the entry.
        pub fn key(&self) -> &K {
            &self.key
        }

        /// Take the ownership of the key and value from the map.
        pub fn remove_entry(self) -> (K, V) {
            let old_value = self.map.remove_from_internal_location(self.location);

            (self.key, old_value.value)
        }

        /// Gets a reference to the value in the entry.
        pub fn get(&self) -> &V {
            let slot_id = self.map.metadata.unwrap_slot(self.location);
            let kv = &self.map.data[slot_id];
            &kv.value
        }

        /// Gets a mutable reference to the value in the entry.
        ///
        /// If you need a reference to the `OccupiedEntry` which may outlive the destruction
        /// of the `Entry` value, see [`into_mut`].
        ///
        /// [`into_mut`]: Self::into_mut
        pub fn get_mut(&mut self) -> &mut V {
            let slot_id = self.map.metadata.unwrap_slot(self.location);
            let kv = &mut self.map.data[slot_id];
            &mut kv.value
        }

        /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry with
        /// a lifetime bound to the map itself.
        ///
        /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
        ///
        /// [`get_mut`]: Self::get_mut
        pub fn into_mut(self) -> &'a mut V {
            let slot_id = self.map.metadata.unwrap_slot(self.location);
            let kv = &mut self.map.data[slot_id];
            &mut kv.value
        }

        /// Sets the value of the entry and returns the entry's old value.
        pub fn insert(&mut self, value: V) -> V {
            let slot_id = self.map.metadata.unwrap_slot(self.location);
            core::mem::replace(&mut self.map.data[slot_id].value, value)
        }

        /// Takes the value out of the entry and returns it.
        pub fn remove(self) -> V {
            self.map.remove_from_internal_location(self.location).value
        }
    }

    /// A view into a vacant entry in a `HashMap`. It is part of the [`crate::Entry`] enum.
    pub struct VacantEntry<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator> {
        key: K,
        map: &'a mut HashMap<K, V, DATA, METADATA>,
        hash: HashType,
    }

    impl<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator + Copy>
        VacantEntry<'a, K, V, DATA, METADATA>
    {
        pub(crate) fn new(
            key: K,
            map: &'a mut HashMap<K, V, DATA, METADATA>,
            hash: HashType,
        ) -> Self {
            Self { key, map, hash }
        }

        /// Gets a reference to the key that would be used when inserting a value through `VacantEntry`
        pub fn key(&self) -> &K {
            &self.key
        }

        /// Take ownership of the key
        pub fn into_key(self) -> K {
            self.key
        }

        /// Sets the value of the entry with the `VacantEntry`'s key and returns a mutable reference to it.
        pub fn insert(self, value: V) -> &'a mut V
        where
            K: Hash + Eq,
        {
            self.map.insert_new(self.hash, self.key, value)
        }
    }
}

pub use entries::{OccupiedEntry, VacantEntry};

use crate::metadata::{Metadata, Resize, SlotId};

/// A view into a single entry in a map, which may be vacant or occupied.
///
/// This is constructed using the [`entry`] method on [`HashMap`]
///
/// [`entry`]: HashMap::entry()
pub enum Entry<'a, K: 'a, V: 'a, DATA: Allocator, METADATA: Allocator> {
    /// An occupied entry
    Occupied(OccupiedEntry<'a, K, V, DATA, METADATA>),
    /// A vacant entry
    Vacant(VacantEntry<'a, K, V, DATA, METADATA>),
}

impl<'a, K, V, DATA: Allocator, METADATA: Allocator + Copy> Entry<'a, K, V, DATA, METADATA>
where
    K: Hash + Eq,
{
    /// Ensures a value is in the entry by inserting the given value, and returns a mutable
    /// reference to the value in the entry.
    pub fn or_insert(self, value: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(value),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the function if empty, and
    /// returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F>(self, f: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the function if empty, and
    /// returns a mutable reference to the value in the entry. This method allows for key-derived
    /// values for insertion by providing the function with a reference to the key.
    ///
    /// The reference to the moved key is provided so that cloning or copying the key is unnecessary,
    /// unlike with `.or_insert_with(|| ...)`.
    pub fn or_insert_with_key<F>(self, f: F) -> &'a mut V
    where
        F: FnOnce(&K) -> V,
    {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let value = f(e.key());
                e.insert(value)
            }
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts
    /// into the map.
    #[must_use]
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            Entry::Vacant(e) => Entry::Vacant(e),
        }
    }

    /// Ensures a value is in th entry by inserting the default value if empty. Returns a
    /// mutable reference to the value in the entry.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(Default::default()),
        }
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }
}

impl<K, V, DATA: Allocator, METADATA: Allocator + Copy> HashMap<K, V, DATA, METADATA>
where
    K: Hash + Eq,
{
    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, DATA, METADATA> {
        let hash = hash(&key);
        let location = self.metadata.find_entry_matching(hash, |location| {
            let slot_id = self.metadata.unwrap_slot(location);
            self.data[slot_id].key == key
        });

        if let Some(location) = location {
            Entry::Occupied(OccupiedEntry::new(key, self, location))
        } else {
            Entry::Vacant(VacantEntry::new(key, self, hash))
        }
    }
}

impl<K, V> FromIterator<(K, V)> for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = HashMap::new();
        map.extend(iter);
        map
    }
}

impl<K, V> Extend<(K, V)> for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K, V, Q, ALLOCATOR: Allocator> Index<&Q> for HashMap<K, V, ALLOCATOR>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<K, V, ALLOCATOR: Allocator> PartialEq for HashMap<K, V, ALLOCATOR>
where
    K: Eq + Hash,
    V: PartialEq,
{
    fn eq(&self, other: &HashMap<K, V, ALLOCATOR>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter()
            .all(|(key, value)| other.get(key).is_some_and(|v| *value == *v))
    }
}

impl<K, V, ALLOCATOR: Allocator> Eq for HashMap<K, V, ALLOCATOR>
where
    K: Eq + Hash,
    V: PartialEq,
{
}

impl<K, V, ALLOCATOR: Allocator> Debug for HashMap<K, V, ALLOCATOR>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct HashType(u32);

impl From<usize> for HashType {
    fn from(value: usize) -> Self {
        // we explicitly want to allow truncation
        #[allow(clippy::cast_possible_truncation)]
        Self(value as u32)
    }
}

impl HashType {
    // 32 bit mix function from here: https://github.com/skeeto/hash-prospector
    fn bit_mix(key: u32) -> Self {
        let mut key = Wrapping(key);
        key ^= key >> 16;
        key *= 0x7feb352d;
        key ^= key >> 15;
        key *= 0x846ca68b;
        key ^= key >> 16;

        Self(key.0)
    }

    pub(crate) fn fast_mod(self, len: usize) -> usize {
        debug_assert!(len.is_power_of_two(), "Length must be a power of 2");
        (self.0 as usize) & (len - 1)
    }
}

impl core::ops::Add<i32> for HashType {
    type Output = HashType;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0.wrapping_add_signed(rhs))
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::missing_panics_doc)]
    use core::{cell::RefCell, hash::Hasher};

    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn can_create_with_size() {
        let map: HashMap<i32, i32> = HashMap::with_size_in(2, Global, Global);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn can_create_with_capacity() {
        let map: HashMap<i32, i32> = HashMap::with_capacity_in(0, Global, Global);
        assert_eq!(map.len(), 0);
    }

    #[test]
    #[should_panic]
    fn cannot_create_with_overflows() {
        let _ = HashMap::<i32, i32>::with_capacity_in((1 << 31) * 3 / 5, Global, Global);
    }

    #[test]
    fn can_store_and_retrieve_8_elements() {
        let mut map = HashMap::new();

        for i in 0..8 {
            map.insert(i, i % 4);
        }

        for i in 0..8 {
            assert_eq!(map.get(&i), Some(&(i % 4)));
        }
    }

    #[test]
    fn can_get_the_length() {
        let mut map = HashMap::new();

        for i in 0..8 {
            map.insert(i / 2, true);
        }

        assert_eq!(map.len(), 4);
    }

    #[test]
    fn returns_none_if_element_does_not_exist() {
        let mut map = HashMap::new();

        for i in 0..8 {
            map.insert(i, i % 3);
        }

        assert_eq!(map.get(&12), None);
    }

    #[test]
    fn can_delete_entries() {
        let mut map = HashMap::new();

        for i in 0..8 {
            map.insert(i, i % 3);
        }

        for i in 0..4 {
            map.remove(&i);
        }

        assert_eq!(map.len(), 4);
        assert_eq!(map.get(&3), None);
        assert_eq!(map.get(&7), Some(&1));
    }

    #[test]
    fn can_iterate_through_all_entries() {
        let mut map = HashMap::new();

        for i in 0..8 {
            map.insert(i, i);
        }

        let mut max_found = -1;
        let mut num_found = 0;

        for (_, value) in map {
            max_found = max_found.max(value);
            num_found += 1;
        }

        assert_eq!(num_found, 8);
        assert_eq!(max_found, 7);
    }

    #[test]
    fn can_insert_more_than_initial_capacity() {
        let mut map = HashMap::new();

        for i in 0..65 {
            map.insert(i, i % 4);
        }

        for i in 0..65 {
            assert_eq!(map.get(&i), Some(&(i % 4)));
        }
    }

    #[test]
    fn can_entry_or_insert() {
        let mut map = HashMap::new();
        map.insert(1, 10);
        let v = map.entry(1).or_insert(20);
        assert_eq!(*v, 10);
    }

    #[test]
    fn can_entry_or_insert_with() {
        let mut map = HashMap::new();
        map.insert(3, 15);
        let v = map.entry(3).or_insert_with(|| 25);
        assert_eq!(*v, 15);
    }

    #[test]
    fn can_entry_or_insert_with_key() {
        let mut map = HashMap::new();
        map.insert(5, 50);
        let v = map.entry(5).or_insert_with_key(|k| k * 10);
        assert_eq!(*v, 50);
    }

    #[test]
    fn can_entry_or_default_for_existing_key() {
        let mut map = HashMap::new();
        map.insert(9, 9);
        let v = map.entry(9).or_default();
        assert_eq!(*v, 9);
    }

    #[test]
    fn can_entry_or_default_for_missing_key() {
        let mut map = HashMap::new();
        let v = map.entry(10).or_default();
        assert_eq!(*v, 0);
        assert_eq!(map.get(&10), Some(&0));
    }

    struct NoisyDrop {
        i: i32,
        dropped: bool,
    }

    impl NoisyDrop {
        #[cfg(not(miri))]
        fn new(i: i32) -> Self {
            Self { i, dropped: false }
        }
    }

    impl PartialEq for NoisyDrop {
        fn eq(&self, other: &Self) -> bool {
            self.i == other.i
        }
    }

    impl Eq for NoisyDrop {}

    impl Hash for NoisyDrop {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            hasher.write_i32(self.i);
        }
    }

    impl Drop for NoisyDrop {
        fn drop(&mut self) {
            assert!(!self.dropped, "NoisyDropped dropped twice");

            self.dropped = true;
        }
    }

    trait RngNextI32 {
        fn next_i32(&mut self) -> i32;
    }

    impl<T> RngNextI32 for T
    where
        T: rand::Rng,
    {
        fn next_i32(&mut self) -> i32 {
            self.next_u32() as i32
        }
    }

    #[cfg(not(miri))] // takes way too long to run under miri
    #[test]
    fn extreme_case() {
        use rand::SeedableRng;

        let mut map = HashMap::new();
        let mut rng = rand::rngs::SmallRng::seed_from_u64(20);

        let mut answers: [Option<i32>; 512] = [None; 512];

        for _ in 0..15_000 {
            let command = rng.next_i32().rem_euclid(2);
            let key = rng.next_i32().rem_euclid(answers.len().try_into().unwrap());
            let value = rng.next_i32();

            match command {
                0 => {
                    // insert
                    answers[key as usize] = Some(value);
                    map.insert(NoisyDrop::new(key), NoisyDrop::new(value));
                }
                1 => {
                    // remove
                    answers[key as usize] = None;
                    map.remove(&NoisyDrop::new(key));
                }
                _ => {}
            }

            for (i, answer) in answers.iter().enumerate() {
                assert_eq!(
                    map.get(&NoisyDrop::new(i.try_into().unwrap()))
                        .map(|nd| &nd.i),
                    answer.as_ref()
                );
            }

            assert!(map.capacity() >= map.len());
            map.metadata.assert_invariants();
        }
    }

    #[derive(Clone)]
    struct Droppable<'a> {
        id: usize,
        drop_registry: &'a DropRegistry,
    }

    impl Hash for Droppable<'_> {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            hasher.write_usize(self.id);
        }
    }

    impl PartialEq for Droppable<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Eq for Droppable<'_> {}

    impl Drop for Droppable<'_> {
        fn drop(&mut self) {
            self.drop_registry.dropped(self.id);
        }
    }

    struct DropRegistry {
        are_dropped: RefCell<Vec<i32>>,
    }

    impl DropRegistry {
        fn new() -> Self {
            Self {
                are_dropped: RefCell::default(),
            }
        }

        fn new_droppable(&self) -> Droppable<'_> {
            self.are_dropped.borrow_mut().push(0);
            Droppable {
                id: self.are_dropped.borrow().len() - 1,
                drop_registry: self,
            }
        }

        fn dropped(&self, id: usize) {
            self.are_dropped.borrow_mut()[id] += 1;
        }

        fn assert_dropped_once(&self, id: usize) {
            assert_eq!(self.are_dropped.borrow()[id], 1);
        }

        fn assert_not_dropped(&self, id: usize) {
            assert_eq!(self.are_dropped.borrow()[id], 0);
        }

        fn assert_dropped_n_times(&self, id: usize, num_drops: i32) {
            assert_eq!(self.are_dropped.borrow()[id], num_drops);
        }
    }

    #[test]
    fn correctly_drops_on_remove_and_overall_drop() {
        let drop_registry = DropRegistry::new();

        let droppable1 = drop_registry.new_droppable();
        let droppable2 = drop_registry.new_droppable();

        let id1 = droppable1.id;
        let id2 = droppable2.id;

        {
            let mut map = HashMap::new();

            map.insert(1, droppable1);
            map.insert(2, droppable2);

            drop_registry.assert_not_dropped(id1);
            drop_registry.assert_not_dropped(id2);

            map.remove(&1);
            drop_registry.assert_dropped_once(id1);
            drop_registry.assert_not_dropped(id2);
        }

        drop_registry.assert_dropped_once(id2);
    }

    #[test]
    fn correctly_drop_on_override() {
        let drop_registry = DropRegistry::new();

        let droppable1 = drop_registry.new_droppable();
        let droppable2 = drop_registry.new_droppable();

        let id1 = droppable1.id;
        let id2 = droppable2.id;

        {
            let mut map = HashMap::new();

            map.insert(1, droppable1);
            drop_registry.assert_not_dropped(id1);
            map.insert(1, droppable2);

            drop_registry.assert_dropped_once(id1);
            drop_registry.assert_not_dropped(id2);
        }

        drop_registry.assert_dropped_once(id2);
    }

    #[test]
    fn correctly_drops_key_on_override() {
        let drop_registry = DropRegistry::new();

        let droppable1 = drop_registry.new_droppable();
        let droppable1a = droppable1.clone();

        let id1 = droppable1.id;

        {
            let mut map = HashMap::new();

            map.insert(droppable1, 1);
            drop_registry.assert_not_dropped(id1);
            map.insert(droppable1a, 2);

            drop_registry.assert_dropped_once(id1);
        }

        drop_registry.assert_dropped_n_times(id1, 2);
    }

    #[test]
    fn test_value_mut() {
        let mut map = HashMap::new();
        map.insert("x", 1);
        for v in map.values_mut() {
            *v += 1;
        }
        assert_eq!(map.get(&"x"), Some(&2));
    }

    #[test]
    fn test_iter_mut() {
        let mut map = HashMap::new();
        map.insert(1, 1);
        map.insert(2, 2);
        for (k, v) in map.iter_mut() {
            *v += *k;
        }
        assert_eq!(map.get(&1), Some(&2));
        assert_eq!(map.get(&2), Some(&4));
    }

    #[test]
    fn test_retain() {
        let mut map = HashMap::new();

        for i in 0..100 {
            map.insert(i, i);
        }

        map.retain(|k, _| k % 2 == 0);

        assert_eq!(map[&2], 2);
        assert_eq!(map.get(&3), None);

        assert_eq!(map.iter().count(), 50); // force full iteration
    }

    #[test]
    fn test_size_hint_iter() {
        let mut map = HashMap::new();

        for i in 0..100 {
            map.insert(i, i);
        }

        let mut iter = map.iter();
        assert_eq!(iter.size_hint(), (100, Some(100)));

        iter.next();

        assert_eq!(iter.size_hint(), (99, Some(99)));
    }

    #[test]
    fn test_size_hint_into_iter() {
        let mut map = HashMap::new();

        for i in 0..100 {
            map.insert(i, i);
        }

        let mut iter = map.into_iter();
        assert_eq!(iter.size_hint(), (100, Some(100)));

        iter.next();

        assert_eq!(iter.size_hint(), (99, Some(99)));
    }

    #[test]
    fn clear_should_empty_hashmap() {
        let mut map = HashMap::new();
        assert_eq!(map.len(), 0);

        assert!(map.is_empty());

        map.insert(5, 0);
        map.insert(4, 3);

        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());

        map.clear();

        assert_eq!(map.len(), 0);
        assert!(map.is_empty());

        let values = map.values().collect::<Vec<_>>();
        assert_eq!(values, Vec::<&i32>::new());
    }

    #[test]
    fn clear_should_not_decrease_capacity() {
        let mut map = HashMap::new();

        for i in 0..64 {
            map.insert(i, i + 1);
        }

        let before_clear_capacity = map.capacity();
        assert!(before_clear_capacity >= 64);

        map.clear();
        assert!(map.capacity() == before_clear_capacity);
    }

    // Following test cases copied from the rust source
    // https://github.com/rust-lang/rust/blob/master/library/std/src/collections/hash/map/tests.rs
    mod rust_std_tests {
        use alloc::format;

        use crate::{Entry::*, HashMap};

        #[test]
        fn test_entry() {
            let xs = [(1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)];

            let mut map: HashMap<_, _> = xs.iter().copied().collect();

            // Existing key (insert)
            match map.entry(1) {
                Vacant(_) => unreachable!(),
                Occupied(mut view) => {
                    assert_eq!(view.get(), &10);
                    assert_eq!(view.insert(100), 10);
                }
            }
            assert_eq!(map.get(&1).unwrap(), &100);
            assert_eq!(map.len(), 6);

            // Existing key (update)
            match map.entry(2) {
                Vacant(_) => unreachable!(),
                Occupied(mut view) => {
                    let v = view.get_mut();
                    let new_v = (*v) * 10;
                    *v = new_v;
                }
            }
            assert_eq!(map.get(&2).unwrap(), &200);
            assert_eq!(map.len(), 6);

            // Existing key (take)
            match map.entry(3) {
                Vacant(_) => unreachable!(),
                Occupied(view) => {
                    assert_eq!(view.remove(), 30);
                }
            }
            assert_eq!(map.get(&3), None);
            assert_eq!(map.len(), 5);

            // Inexistent key (insert)
            match map.entry(10) {
                Occupied(_) => unreachable!(),
                Vacant(view) => {
                    assert_eq!(*view.insert(1000), 1000);
                }
            }
            assert_eq!(map.get(&10).unwrap(), &1000);
            assert_eq!(map.len(), 6);
        }

        #[test]
        fn test_occupied_entry_key() {
            let mut a = HashMap::new();
            let key = "hello there";
            let value = "value goes here";
            assert!(a.is_empty());
            a.insert(key, value);
            assert_eq!(a.len(), 1);
            assert_eq!(a[key], value);

            match a.entry(key) {
                Vacant(_) => panic!(),
                Occupied(e) => assert_eq!(key, *e.key()),
            }
            assert_eq!(a.len(), 1);
            assert_eq!(a[key], value);
        }

        #[test]
        fn test_vacant_entry_key() {
            let mut a = HashMap::new();
            let key = "hello there";
            let value = "value goes here";

            assert!(a.is_empty());
            match a.entry(key) {
                Occupied(_) => panic!(),
                Vacant(e) => {
                    assert_eq!(key, *e.key());
                    e.insert(value);
                }
            }
            assert_eq!(a.len(), 1);
            assert_eq!(a[key], value);
        }

        #[test]
        fn test_index() {
            let mut map = HashMap::new();

            map.insert(1, 2);
            map.insert(2, 1);
            map.insert(3, 4);

            assert_eq!(map[&2], 1);
        }

        #[test]
        fn test_eq() {
            let mut m1 = HashMap::new();
            m1.insert(1, 2);
            m1.insert(2, 3);
            m1.insert(3, 4);

            let mut m2 = HashMap::new();
            m2.insert(1, 2);
            m2.insert(2, 3);

            assert!(m1 != m2);

            m2.insert(3, 4);

            assert_eq!(m1, m2);
        }

        #[test]
        fn test_show() {
            let mut map = HashMap::new();
            let empty: HashMap<i32, i32> = HashMap::new();

            map.insert(1, 2);
            map.insert(3, 4);

            let map_str = format!("{map:?}");

            assert!(map_str == "{1: 2, 3: 4}" || map_str == "{3: 4, 1: 2}");
            assert_eq!(format!("{empty:?}"), "{}");
        }
    }

    #[test]
    fn test_fast_mod() {
        let h = HashType(10);
        assert_eq!(h.fast_mod(8), 2);
    }

    #[cfg(not(miri))]
    quickcheck::quickcheck! {
        fn test_against_btree_map(entries: Vec<(u8, u32)>) -> bool {
            let std_hashmap = alloc::collections::BTreeMap::from_iter(entries.clone());
            let agb_hashmap = HashMap::from_iter(entries);

            if std_hashmap.len() != agb_hashmap.len() {
                return false;
            }

            std_hashmap.iter().all(|(key, value)| agb_hashmap.get(key) == Some(value)) &&
            agb_hashmap.iter().all(|(key, value)| std_hashmap.get(key) == Some(value))
        }
    }
}
