// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! High-level memory management.
use crate::{Memory, MemoryBuilder, Trail, TrailBuilder};

/// Representation of how something is stored on the trail.
///
/// Objects can be stored on the trail in trailed or stable memory, represented by
/// [`Trailed`](Trailed) and [`Stable`](Stable), respectively. Both of these structs implement
/// `StorageMode`. See the documentation for [`Trail`](Trail) for the difference between trailed
/// and stable storage.
pub trait StorageMode {
    /// Returns the associated `MemoryBuilder` from a `TrailBuilder`.
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder;

    /// Returns the associated `Memory` from a `Trail`.
    fn memory(trail: &Trail) -> &Memory;

    /// Returns the associated `Memory` from a `Trail` mutably.
    fn memory_mut(trail: &mut Trail) -> &mut Memory;
}

/// Objects stored on the trail in trailed memory.
///
/// Instead of using `Trailed` directly, it's often easier to use the type definitions
/// [`TrailedValue`](crate::TrailedValue) and [`TrailedArray`](crate::TrailedArray).
///
/// # Examples
///
/// ```
/// use contrail::{TrailBuilder, TrailedValue};
///
/// let mut builder = TrailBuilder::new();
/// let trailed_counter = TrailedValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// assert_eq!(trailed_counter.get(&trail), 0);
///
/// trail.new_level();
///
/// trailed_counter.update(&mut trail, |x| x + 1);
///
/// assert_eq!(trailed_counter.get(&trail), 1);
///
/// trail.backtrack();
///
/// assert_eq!(trailed_counter.get(&trail), 0);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Trailed;

impl StorageMode for Trailed {
    #[inline(always)]
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder {
        &mut builder.trailed_mem
    }

    #[inline(always)]
    fn memory(trail: &Trail) -> &Memory {
        &trail.trailed_mem
    }

    #[inline(always)]
    fn memory_mut(trail: &mut Trail) -> &mut Memory {
        &mut trail.trailed_mem
    }
}

/// Objects stored on the trail in stable memory.
///
/// Instead of using `Stable` directly, it's often easier to use the type definitions
/// [`StableValue`](crate::StableValue) and [`StableArray`](crate::StableArray).
///
/// # Examples
///
/// ```
/// use contrail::{StableValue, TrailBuilder};
///
/// let mut builder = TrailBuilder::new();
/// let stable_counter = StableValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// assert_eq!(stable_counter.get(&trail), 0);
///
/// trail.new_level();
///
/// stable_counter.update(&mut trail, |x| x + 1);
///
/// assert_eq!(stable_counter.get(&trail), 1);
///
/// trail.backtrack();
///
/// assert_eq!(stable_counter.get(&trail), 1);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Stable;

impl StorageMode for Stable {
    #[inline(always)]
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder {
        &mut builder.stable_mem
    }

    #[inline(always)]
    fn memory(trail: &Trail) -> &Memory {
        &trail.stable_mem
    }

    #[inline(always)]
    fn memory_mut(trail: &mut Trail) -> &mut Memory {
        &mut trail.stable_mem
    }
}
