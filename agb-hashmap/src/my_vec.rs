use core::ops::{Deref, DerefMut};

use alloc::vec::Vec;

use crate::{Allocator, Global};

pub(crate) use inner::MyVec;

#[cfg(not(feature = "allocator_api"))]
mod inner {
    use super::*;

    #[derive(Clone)]
    pub(crate) struct MyVec<T, A: Allocator = Global>(Vec<T>, A);

    impl<T: core::fmt::Debug, A: Allocator> core::fmt::Debug for MyVec<T, A> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_list().entries(&self.0).finish()
        }
    }

    impl<T, A: Allocator> MyVec<T, A> {
        pub(crate) fn with_capacity_in(capacity: usize, allocator: A) -> Self {
            Self(Vec::with_capacity(capacity), allocator)
        }

        pub(crate) const fn new_in(allocator: A) -> Self {
            Self(Vec::new(), allocator)
        }

        pub(crate) fn allocator(&self) -> &A {
            &self.1
        }
    }
    impl<T, A: Allocator> Deref for MyVec<T, A> {
        type Target = Vec<T>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T, A: Allocator> DerefMut for MyVec<T, A> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(feature = "allocator_api")]
mod inner {
    use super::*;

    #[derive(Clone)]
    pub(crate) struct MyVec<T, A: Allocator = Global>(Vec<T, A>);

    impl<T: core::fmt::Debug, A: Allocator> core::fmt::Debug for MyVec<T, A> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_list().entries(&self.0).finish()
        }
    }

    impl<T, A: Allocator> MyVec<T, A> {
        pub(crate) fn with_capacity_in(capacity: usize, allocator: A) -> Self {
            Self(Vec::with_capacity_in(capacity, alloc))
        }

        pub(crate) const fn new_in(allocator: A) -> Self {
            Self(Vec::new_in(allocator))
        }
    }

    impl<T, A: Allocator> Deref for MyVec<T, A> {
        type Target = Vec<T, A>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T, A: Allocator> DerefMut for MyVec<T, A> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
