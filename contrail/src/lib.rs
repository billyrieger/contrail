/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Simple state management for backtracking search algorithms.
//!
//! During a typical branching search algorithm, the search state must be cloned at each branch
//! point in order to explore the branches independently. `contrail` provides a framework to create
//! search algorithms that only require a partial clone of the search state at each branch point.
//! This is facilitated by the [_trail_](Trail), a struct where all search state is stored.
//!
//! This library is heavily based on the memory model used by
//! [Minion](https://constraintmodelling.org/minion/), a C++ constraint satisfaction problem
//! solver.
//!
//! # Warning
//!
//! A `Value` or an `Array` is only usable with the `Trail` from the `TrailBuilder` used to create
//! it. For instance, if there are multiple trails, it's undefined behavior to use a `Value`
//! created from one trail with another trail.
#[allow(unused_imports)]
#[macro_use]
extern crate contrail_derive;
#[doc(hidden)]
pub use contrail_derive::*;

pub mod mem;
pub mod storage;

use std::{fmt, marker::PhantomData};

use crate::{
    mem::{ArrayPointer, Bytes, Memory, MemoryBuilder, Pointer},
    storage::{Backtrackable, NonBacktrackable, StorageMode},
};

/// The trail itself.
///
/// # Backtrackable and non-backtrackable memory
///
/// The trail consists of [_backtrackable_](storage::Backtrackable) memory and
/// [_non-backtrackable_](storage::NonBacktrackable) memory. Both types of storage can be used with
/// [`Value`](Value) and [`Array`](Array). Whenever `trail.new_level()` is called, a clone of the
/// backtrackable memory is made and appended to an internal stack.  Conversely, whenever
/// `trail.backtrack()` is called, the current backtrackable memory is replaced with the most
/// recent clone from the internal stack.  Non-backtrackable memory is unaffected by these methods.
///
/// When designing data structures using the trail, try to store as much as possible in
/// non-backtrackable storage. This will make calls to `new_level()` and `backtrack()` more
/// efficient.
///
/// # Examples
///
/// The following example illustrates the differences between `Backtrackable` and
/// `NonBacktrackable` storage:
///
/// ```
/// use contrail::{BacktrackableValue, NonBacktrackableValue, TrailBuilder};
///
/// let mut builder = TrailBuilder::new();
/// let backtrackable_counter = BacktrackableValue::new(&mut builder, 0);
/// let non_backtrackable_counter = NonBacktrackableValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// assert_eq!(backtrackable_counter.get(&trail), 0);
/// assert_eq!(non_backtrackable_counter.get(&trail), 0);
///
/// trail.new_level();
///
/// backtrackable_counter.update(&mut trail, |x| x + 1);
/// non_backtrackable_counter.update(&mut trail, |x| x + 1);
///
/// assert_eq!(backtrackable_counter.get(&trail), 1);
/// assert_eq!(non_backtrackable_counter.get(&trail), 1);
///
/// trail.backtrack();
///
/// assert_eq!(backtrackable_counter.get(&trail), 0);
/// assert_eq!(non_backtrackable_counter.get(&trail), 1);
/// ```
///
/// Another example that backtracks multiple times:
///
/// ```
/// use contrail::{BacktrackableValue, TrailBuilder};
///
/// let mut builder = TrailBuilder::new();
/// let countdown = BacktrackableValue::new(&mut builder, 3);
/// let mut trail = builder.finish();
///
/// println!("Counting down from {}:", countdown.get(&trail));
///
/// while countdown.get(&trail) > 0 {
///     trail.new_level();
///     println!("{}...", countdown.get(&trail));
///     countdown.update(&mut trail, |x| x - 1);
/// }
///
/// println!("{}!", countdown.get(&trail));
///
/// println!("Counting back up:");
///
/// while !trail.is_trail_empty() {
///     trail.backtrack();
///     println!("{}", countdown.get(&trail));
/// }
/// ```
///
/// This produces the following output:
///
/// ```txt
/// Counting down from 3:
/// 3...
/// 2...
/// 1...
/// 0!
/// Counting back up:
/// 1
/// 2
/// 3
/// ```
pub struct Trail {
    backtrackable_mem: Memory,
    non_backtrackable_mem: Memory,
    trail: Vec<Memory>,
}

impl Trail {
    /// Adds a new level to the trail.
    ///
    /// When this method is called, a clone of the trail's backtrackable memory at that point in
    /// time is added to an internal stack of memory. These memory snapshots can be recalled in
    /// FILO order using [`backtrack()`](Trail::backtrack).
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 0);
    /// let mut trail = builder.finish();
    ///
    /// value.set(&mut trail, 1);
    /// trail.new_level();
    /// value.set(&mut trail, 2);
    /// trail.backtrack();
    /// assert_eq!(value.get(&trail), 1);
    /// ```
    pub fn new_level(&mut self) {
        self.trail.push(self.backtrackable_mem.clone());
    }

    /// Backtracks the trail to the most recent level.
    ///
    /// When this method is called, the most recent backtrackable memory stored in the trail's
    /// internal stack is removed from the stack and set as the current backtrackable memory. If
    /// the trail is empty, this method has no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 0);
    /// let mut trail = builder.finish();
    ///
    /// value.set(&mut trail, 1);
    /// trail.new_level();
    /// value.set(&mut trail, 2);
    /// trail.backtrack();
    /// assert_eq!(value.get(&trail), 1);
    /// ```
    pub fn backtrack(&mut self) {
        if let Some(prev) = self.trail.pop() {
            self.backtrackable_mem = prev;
        }
    }

    /// Returns the length of the trail.
    ///
    /// The length of the trail is increased whenever a level is added, and decreased whenever a
    /// backtrack occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    ///
    /// let mut trail = TrailBuilder::new().finish();
    ///
    /// assert_eq!(trail.trail_len(), 0);
    ///
    /// trail.new_level();
    /// assert_eq!(trail.trail_len(), 1);
    ///
    /// trail.backtrack();
    /// assert_eq!(trail.trail_len(), 0);
    /// ```
    pub fn trail_len(&self) -> usize {
        self.trail.len()
    }

    /// Checks if the trail's length is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    ///
    /// let mut trail = TrailBuilder::new().finish();
    ///
    /// assert!(trail.is_trail_empty());
    ///
    /// trail.new_level();
    /// assert!(!trail.is_trail_empty());
    ///
    /// trail.backtrack();
    /// assert!(trail.is_trail_empty());
    /// ```
    pub fn is_trail_empty(&self) -> bool {
        self.trail.is_empty()
    }
}

/// A builder to create a `Trail`.
///
/// # Examples
///
/// ```
/// use contrail::{BacktrackableValue, TrailBuilder};
///
/// let mut builder = TrailBuilder::new();
/// let value = BacktrackableValue::new(&mut builder, 5);
/// let trail = builder.finish();
///
/// assert_eq!(value.get(&trail), 5);
/// ```
#[derive(Debug, Default)]
pub struct TrailBuilder {
    backtrackable_mem: MemoryBuilder,
    non_backtrackable_mem: MemoryBuilder,
}

impl TrailBuilder {
    /// Creates a new empty `TrailBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    ///
    /// let mut builder = TrailBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            backtrackable_mem: MemoryBuilder::new(),
            non_backtrackable_mem: MemoryBuilder::new(),
        }
    }

    /// Consumes the `TrailBuilder` to create a new `Trail`.
    ///
    /// Once this method is called, any `Value` and `Array` that were created using the
    /// `TrailBuilder` are usable with the resulting `Trail`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{NonBacktrackableArray, NonBacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = NonBacktrackableValue::new(&mut builder, -123);
    /// let array = NonBacktrackableArray::new(&mut builder, vec![1, 3, 5, 7]);
    /// let trail = builder.finish();
    ///
    /// assert_eq!(value.get(&trail), -123);
    /// assert_eq!(array.get(&trail, 2), 5);
    /// ```
    pub fn finish(self) -> Trail {
        Trail {
            backtrackable_mem: self.backtrackable_mem.finish(),
            non_backtrackable_mem: self.non_backtrackable_mem.finish(),
            trail: vec![],
        }
    }
}

/// A reference to a value stored on the trail.
///
/// The type parameter `T` is the type of value stored on the trail, and the type parameter `M`
/// represents how the value is stored on the trail.  A `Value<Backtrackable, T>` is stored on the
/// trail in backtrackable memory, whereas a `Value<NonBacktrackable, T>` is stored on the trail in
/// non-backtrackable memory.
///
/// Instead of using `Value` directly, it's often easier to use the type definitions
/// [`BacktrackableValue`](BacktrackableValue) and [`NonBacktrackableValue`](NonBacktrackableValue).
pub struct Value<M, T> {
    pointer: Pointer<T>,
    phantom: PhantomData<M>,
}

impl<M, T> Value<M, T>
where
    M: StorageMode,
    T: Bytes,
{
    /// Creates a new `Value` with the given value.
    ///
    /// The `Value` is usable after the `TrailBuilder` used to create it is finished.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 'b');
    /// let trail = builder.finish();
    ///
    /// // the value is usable now
    /// assert_eq!(value.get(&trail), 'b');
    /// ```
    pub fn new(builder: &mut TrailBuilder, val: T) -> Self {
        Self {
            pointer: Pointer::new(M::builder_mut(builder), val),
            phantom: PhantomData,
        }
    }

    /// Gets the value from the trail.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(value.get(&trail), 5);
    /// ```
    #[inline(always)]
    pub fn get(self, trail: &Trail) -> T {
        self.pointer.get(M::memory(trail))
    }

    /// Sets the value on the trail.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// value.set(&mut trail, 42);
    /// assert_eq!(value.get(&trail), 42);
    /// ```
    #[inline(always)]
    pub fn set(self, trail: &mut Trail, new_val: T) {
        self.pointer.set(M::memory_mut(trail), new_val);
    }

    /// Updates the value on the trail using the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableValue, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let value = BacktrackableValue::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// value.update(&mut trail, |x| x * x);
    /// assert_eq!(value.get(&trail), 25);
    /// ```
    #[inline(always)]
    pub fn update(self, trail: &mut Trail, f: impl FnOnce(T) -> T) {
        self.pointer.update(M::memory_mut(trail), f);
    }
}

impl<M, T> Clone for Value<M, T> {
    fn clone(&self) -> Self {
        Self {
            pointer: self.pointer,
            phantom: PhantomData,
        }
    }
}

impl<M, T> Copy for Value<M, T> {}

impl<M, T> fmt::Debug for Value<M, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Value")
            .field("pointer", &self.pointer)
            .finish()
    }
}

/// A reference to a fixed-length array of values stored on the trail.
///
/// The type parameter `T` is the type of value stored on the trail, and the type parameter `M`
/// represents how the value is stored on the trail.  An `Array<Backtrackable, T>` is stored on the
/// trail in backtrackable memory, whereas an `Array<NonBacktrackable, T>` is stored on the trail
/// in non-backtrackable memory.
///
/// Instead of using `Array` directly, it's often easier to use the type definitions
/// [`BacktrackableArray`](BacktrackableArray) and
/// [`NonBacktrackableArray`](NonBacktrackableArray).
pub struct Array<M, T> {
    pointer: ArrayPointer<T>,
    phantom: PhantomData<M>,
}

impl<M, T> Array<M, T>
where
    M: StorageMode,
    T: Bytes,
{
    /// Creates a new `Array` with the given values.
    ///
    /// The `Array` is usable after the `TrailBuilder` used to create it is finished.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, 5..10);
    /// let trail = builder.finish();
    ///
    /// // the array is usable now
    /// assert_eq!(array.get(&trail, 2), 7);
    /// ```
    pub fn new(builder: &mut TrailBuilder, vals: impl IntoIterator<Item = T>) -> Self {
        Self {
            pointer: ArrayPointer::new(
                M::builder_mut(builder),
                &vals.into_iter().collect::<Vec<_>>(),
            ),
            phantom: PhantomData,
        }
    }

    /// Returns the length of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, 0..8);
    ///
    /// assert_eq!(array.len(), 8);
    /// ```
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.pointer.len()
    }

    /// Checks if the length of the array is equal to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let empty = BacktrackableArray::new(&mut builder, 0..0);
    /// let not_empty = BacktrackableArray::new(&mut builder, 0..1);
    ///
    /// assert_eq!(empty.is_empty(), true);
    /// assert_eq!(not_empty.is_empty(), false);
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.pointer.len() == 0
    }

    /// Returns an iterator over the elements of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let odds = BacktrackableArray::new(&mut builder, (0..10).map(|x| 2 * x + 1));
    /// let trail = builder.finish();
    ///
    /// for odd in odds.iter(&trail) {
    ///     assert_eq!(odd % 2, 1);
    /// }
    /// ```
    pub fn iter<'t>(&self, trail: &'t Trail) -> ArrayIter<'t, M, T> {
        ArrayIter {
            trail,
            index: 0,
            array: *self,
        }
    }

    /// Gets the value of the array at the given index.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, Trail, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, 0..10);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(array.get(&trail, 4), 4);
    /// ```
    #[inline(always)]
    pub fn get(&self, trail: &Trail, i: usize) -> T {
        self.pointer.get(M::memory(trail), i)
    }

    /// Sets the value of the array at the given index.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, Trail, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, 0..10);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(array.get(&trail, 4), 4);
    ///
    /// array.set(&mut trail, 4, -23);
    /// assert_eq!(array.get(&trail, 4), -23);
    /// ```
    #[inline(always)]
    pub fn set(&self, trail: &mut Trail, i: usize, new_val: T) {
        self.pointer.set(M::memory_mut(trail), i, new_val);
    }

    /// Updates the value of the array at the given index using the given update function.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, Trail, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, 0..10);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(array.get(&trail, 4), 4);
    ///
    /// array.update(&mut trail, 4, |x| x * x);
    /// assert_eq!(array.get(&trail, 4), 16);
    /// ```
    #[inline(always)]
    pub fn update(&self, trail: &mut Trail, i: usize, f: impl FnOnce(T) -> T) {
        self.pointer.update(M::memory_mut(trail), i, f);
    }

    /// Swaps the two values at the given indices of the array in memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{BacktrackableArray, TrailBuilder};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let array = BacktrackableArray::new(&mut builder, vec!['r', 'u', 't', 's']);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(array.get(&trail, 2), 't');
    /// assert_eq!(array.get(&trail, 3), 's');
    ///
    /// array.swap(&mut trail, 2, 3);
    ///
    /// assert_eq!(array.get(&trail, 2), 's');
    /// assert_eq!(array.get(&trail, 3), 't');
    /// ```
    #[inline(always)]
    pub fn swap(&self, trail: &mut Trail, i: usize, j: usize) {
        self.pointer.swap(M::memory_mut(trail), i, j);
    }
}

impl<M, T> Clone for Array<M, T> {
    fn clone(&self) -> Self {
        Self {
            pointer: self.pointer,
            phantom: PhantomData,
        }
    }
}

impl<M, T> Copy for Array<M, T> {}

impl<M, T> fmt::Debug for Array<M, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Array")
            .field("pointer", &self.pointer)
            .finish()
    }
}

/// An iterator over the values of an `Array`.
pub struct ArrayIter<'t, M, T> {
    trail: &'t Trail,
    index: usize,
    array: Array<M, T>,
}

impl<'t, M, T> Iterator for ArrayIter<'t, M, T>
where
    M: StorageMode,
    T: Bytes,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index == self.array.len() {
            None
        } else {
            let to_ret = Some(self.array.get(self.trail, self.index));
            self.index += 1;
            to_ret
        }
    }
}

/// A value stored on the trail in backtrackable memory.
pub type BacktrackableValue<T> = Value<Backtrackable, T>;

/// A value stored on the trail in non-backtrackable memory.
pub type NonBacktrackableValue<T> = Value<NonBacktrackable, T>;

/// A fixed-length array stored on the trail in backtrackable memory.
pub type BacktrackableArray<T> = Array<Backtrackable, T>;

/// A fixed-length array stored on the trail in non-backtrackable memory.
pub type NonBacktrackableArray<T> = Array<NonBacktrackable, T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtrackable_and_non_backtrackable_value() {
        let init_val = 5;
        let new_val = 6;

        let mut builder = TrailBuilder::new();
        let backtrackable = BacktrackableValue::new(&mut builder, init_val);
        let non_backtrackable = NonBacktrackableValue::new(&mut builder, init_val);
        let mut trail = builder.finish();

        assert_eq!(trail.trail_len(), 0);
        assert!(trail.is_trail_empty());

        assert_eq!(backtrackable.get(&trail), init_val);
        assert_eq!(non_backtrackable.get(&trail), init_val);

        trail.new_level();
        assert_eq!(trail.trail_len(), 1);
        assert!(!trail.is_trail_empty());

        assert_eq!(backtrackable.get(&trail), init_val);
        assert_eq!(non_backtrackable.get(&trail), init_val);

        backtrackable.set(&mut trail, new_val);
        non_backtrackable.set(&mut trail, new_val);

        assert_eq!(backtrackable.get(&trail), new_val);
        assert_eq!(non_backtrackable.get(&trail), new_val);

        trail.backtrack();
        assert_eq!(trail.trail_len(), 0);
        assert!(trail.is_trail_empty());

        assert_eq!(backtrackable.get(&trail), init_val);
        assert_eq!(non_backtrackable.get(&trail), new_val);
    }

    #[test]
    fn backtrackable_and_non_backtrackable_array() {
        let init_vals = vec![1, 3, 5, 7];
        let new_vals = vec![2, 4, 6, 8];

        let mut builder = TrailBuilder::new();
        let backtrackable = BacktrackableArray::new(&mut builder, init_vals.clone());
        let stored = NonBacktrackableArray::new(&mut builder, init_vals.clone());
        let mut trail = builder.finish();

        assert_eq!(trail.trail_len(), 0);
        assert!(trail.is_trail_empty());

        for i in 0..4 {
            assert_eq!(backtrackable.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), init_vals[i]);

            trail.new_level();
            assert_eq!(trail.trail_len(), 1);
            assert!(!trail.is_trail_empty());

            assert_eq!(backtrackable.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), init_vals[i]);

            backtrackable.set(&mut trail, i, new_vals[i]);
            stored.set(&mut trail, i, new_vals[i]);

            assert_eq!(backtrackable.get(&trail, i), new_vals[i]);
            assert_eq!(stored.get(&trail, i), new_vals[i]);

            trail.backtrack();
            assert_eq!(trail.trail_len(), 0);
            assert!(trail.is_trail_empty());

            assert_eq!(backtrackable.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), new_vals[i]);
        }
    }

    #[test]
    fn array_iter() {
        let vals = vec![1, 3, 5, 7, 9];

        let mut builder = TrailBuilder::new();
        let array = BacktrackableArray::new(&mut builder, vals.clone());
        let trail = builder.finish();

        let iter_vals = array.iter(&trail).collect::<Vec<_>>();
        assert_eq!(iter_vals, vals);
    }
}
