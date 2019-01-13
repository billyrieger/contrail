/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

//! High-level memory management.
use crate::{Memory, MemoryBuilder, Trail, TrailBuilder};

/// Representation of how something is stored on the trail.
///
/// Objects can be stored on the trail in backtrackable or non-backtrackable memory, represented by
/// [`Backtrackable`](Backtrackable) and [`NonBacktrackable`](NonBacktrackable), respectively. Both
/// of these structs implement `StorageMode`. See the documentation for [`Trail`](Trail) for the
/// difference between backtrackable and non-backtrackable storage.
pub trait StorageMode {
    /// Returns the associated `MemoryBuilder` from a `TrailBuilder`.
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder;

    /// Returns the associated `Memory` from a `Trail`.
    fn memory(trail: &Trail) -> &Memory;

    /// Returns the associated `Memory` from a `Trail` mutably.
    fn memory_mut(trail: &mut Trail) -> &mut Memory;
}

/// Objects stored on the trail in backtrackable memory.
///
/// Instead of using `Backtrackable` directly, it's often easier to use the type definitions
/// [`BacktrackableValue`](crate::BacktrackableValue) and
/// [`BacktrackableArray`](crate::NonBacktrackableArray).
///
/// # Examples
///
/// ```
/// use contrail::{TrailBuilder, BacktrackableValue};
///
/// let mut builder = TrailBuilder::new();
/// let backtrackable_counter = BacktrackableValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// assert_eq!(backtrackable_counter.get(&trail), 0);
///
/// trail.new_level();
///
/// backtrackable_counter.update(&mut trail, |x| x + 1);
///
/// assert_eq!(backtrackable_counter.get(&trail), 1);
///
/// trail.backtrack();
///
/// assert_eq!(backtrackable_counter.get(&trail), 0);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Backtrackable;

impl StorageMode for Backtrackable {
    #[inline(always)]
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder {
        &mut builder.backtrackable_mem
    }

    #[inline(always)]
    fn memory(trail: &Trail) -> &Memory {
        &trail.backtrackable_mem
    }

    #[inline(always)]
    fn memory_mut(trail: &mut Trail) -> &mut Memory {
        &mut trail.backtrackable_mem
    }
}

/// Objects stored on the trail in stable memory.
///
/// Instead of using `Stable` directly, it's often easier to use the type definitions
/// [`NonBacktrackableValue`](crate::NonBacktrackableValue) and [`StableArray`](crate::StableArray).
///
/// # Examples
///
/// ```
/// use contrail::{NonBacktrackableValue, TrailBuilder};
///
/// let mut builder = TrailBuilder::new();
/// let non_backtrackable_counter = NonBacktrackableValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// assert_eq!(non_backtrackable_counter.get(&trail), 0);
///
/// trail.new_level();
///
/// non_backtrackable_counter.update(&mut trail, |x| x + 1);
///
/// assert_eq!(non_backtrackable_counter.get(&trail), 1);
///
/// trail.backtrack();
///
/// assert_eq!(non_backtrackable_counter.get(&trail), 1);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NonBacktrackable;

impl StorageMode for NonBacktrackable {
    #[inline(always)]
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder {
        &mut builder.non_backtrackable_mem
    }

    #[inline(always)]
    fn memory(trail: &Trail) -> &Memory {
        &trail.non_backtrackable_mem
    }

    #[inline(always)]
    fn memory_mut(trail: &mut Trail) -> &mut Memory {
        &mut trail.non_backtrackable_mem
    }
}
