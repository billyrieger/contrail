// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! State management for backtracking search algorithms.
//!
//! # Motivation
//!
//! As an illustrative example, consider a recursive search
//! algorithm that looks for a solution to some unspecified problem by
//! exploring a binary tree of possible states:
//!
//! ```
//! trait SearchState: Clone {
//!     fn solution_status(&self) -> Option<bool>;
//!     fn branch_left(&mut self);
//!     fn branch_right(&mut self);
//! }
//!
//! // returns `true` if at least one solution exists, and `false` otherwise.
//! fn search(state: impl SearchState) -> bool {
//!     match state.solution_status() {
//!         Some(is_solution) => is_solution,
//!         None => {
//!             let mut left = state.clone();
//!             left.branch_left();
//!             if search(left) {
//!                 return true;
//!             }
//!
//!             let mut right = state;
//!             right.branch_right();
//!             if search(right) {
//!                 return true;
//!             }
//!
//!             false
//!         }
//!     }
//! }
//! ```
//!
//! The main issue with this implementation is the line `let mut left =
//! state.clone()`. The clone is necessary because the methods `branch_left`
//! and `branch_right` both mutably modify the search state.
//! If the search state is large in memory, cloning the entire state at
//! each branch point could be expensive and slow down the search.
//!
//! `contrail` solves this problem by providing a framework for only cloning
//! the relevant parts of the search state at each branch point.
//! To enable this, all state is stored on an object called the _trail_.
//! Here's what the same search procedure looks like using `contrail`:
//!
//! ```
//! use contrail::Trail;
//!
//! trait SearchState {
//!     fn solution_status(&self, trail: &Trail) -> Option<bool>;
//!     fn branch_left(&self, trail: &mut Trail);
//!     fn branch_right(&self, trail: &mut Trail);
//! }
//!
//! // returns `true` if at least one solution exists, and `false` otherwise.
//! fn search(trail: &mut Trail, state: &impl SearchState) -> bool {
//!     match state.solution_status(trail) {
//!         Some(is_solution) => is_solution,
//!         None => {
//!             trail.new_level();
//!             state.branch_left(trail);
//!             if search(trail, state) {
//!                 return true;
//!             }
//!             trail.backtrack();
//!
//!             trail.new_level();
//!             state.branch_right(trail);
//!             if search(trail, state) {
//!                 return true;
//!             }
//!             trail.backtrack();
//!
//!             false
//!         }
//!     }
//! }
//! ```
//!
//! Some key differences from the original example:
//!
//! - Functions and methods take a reference to a `Trail` as their first
//! parameter. This is extremely common in `contrail` since the trail stores
//! the entire state.
//!
//! - `branch_left` and `branch_right` no longer require `&mut self`, since the
//! search state is stored on the trail.
//!
//! - In the `search` function, instead of cloning the search state,
//! a new level is pushed onto the trail before the state is modified.
//! Once the recursive sub-search is complete, the state is restored to
//! when the new level was added.
#[allow(unused_imports)]
#[macro_use]
extern crate contrail_derive;
#[doc(hidden)]
pub use contrail_derive::*;

pub mod mem;

use std::marker::PhantomData;

use self::mem::{ArrayPointer, Bytes, Memory, MemoryBuilder, Pointer};

/// The trail itself.
///
/// A `Trail`
///
/// # Creating a trail
///
/// The only way to create a trail is using a `TrailBuilder`.
/// See [its documentation](crate::TrailBuilder) for more.
///
/// # Examples
///
/// The difference between trailed memory and stable memory is demonstrated by
/// the following example:
///
/// ```
/// use contrail::{StableValue, TrailBuilder, TrailedValue};
///
/// // setup the trail
/// let mut builder = TrailBuilder::new();
/// let trailed_counter = TrailedValue::new(&mut builder, 0);
/// let stable_counter = StableValue::new(&mut builder, 0);
/// let mut trail = builder.finish();
///
/// // both counters start at 0
/// assert_eq!(trailed_counter.get(&trail), 0);
/// assert_eq!(stable_counter.get(&trail), 0);
///
/// // push a new level onto the trail
/// trail.new_level();
///
/// // increment each counter
/// trailed_counter.update(&mut trail, |x| x + 1);
/// stable_counter.update(&mut trail, |x| x + 1);
///
/// // both counters are now at 1
/// assert_eq!(trailed_counter.get(&trail), 1);
/// assert_eq!(stable_counter.get(&trail), 1);
///
/// // reset the trail to the most recent level
/// trail.backtrack();
///
/// // the trailed counter is reset
/// // the stable counter is unchanged
/// assert_eq!(trailed_counter.get(&trail), 0);
/// assert_eq!(stable_counter.get(&trail), 1);
/// ```
///
/// ```
/// use contrail::{TrailBuilder, TrailedValue};
///
/// // for writeln!
/// use std::fmt::Write;
///
/// // instead of writing to stdout with println!,
/// // capture the output with writeln! to verify it at the end.
/// let mut output = String::new();
/// writeln!(output, "output:");
///
/// let mut builder = TrailBuilder::new();
/// let countdown = TrailedValue::new(&mut builder, 3);
/// let mut trail = builder.finish();
///
/// writeln!(output, "Counting down from {}:", countdown.get(&trail));
///
/// while countdown.get(&trail) > 0 {
///     trail.new_level();
///     writeln!(output, "{}...", countdown.get(&trail));
///     countdown.update(&mut trail, |x| x - 1);
/// }
///
/// writeln!(output, "{}!", countdown.get(&trail));
///
/// writeln!(output, "Counting back up:");
///
/// while !trail.is_trail_empty() {
///     trail.backtrack();
///     writeln!(output, "{}", countdown.get(&trail));
/// }
///
/// assert_eq!(
///     output,
///     "output:
/// Counting down from 3:
/// 3...
/// 2...
/// 1...
/// 0!
/// Counting back up:
/// 1
/// 2
/// 3
/// "
///     .to_owned()
/// );
/// ```
pub struct Trail {
    trailed_mem: Memory,
    stable_mem: Memory,
    trail: Vec<Memory>,
}

impl Trail {
    pub fn new_level(&mut self) {
        self.trail.push(self.trailed_mem.clone());
    }

    pub fn backtrack(&mut self) {
        if let Some(prev) = self.trail.pop() {
            self.trailed_mem = prev;
        }
    }

    /// Returns the length of the trail.
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

    /// Checks if the trail is empty.
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

/// Trait representing how something is stored on the trail.
pub trait StorageMode {
    fn builder_mut(builder: &mut TrailBuilder) -> &mut MemoryBuilder;
    fn memory(trail: &Trail) -> &Memory;
    fn memory_mut(trail: &mut Trail) -> &mut Memory;
}

/// Objects stored on the trail in trailed memory.
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
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

/// A builder to create a `Trail`.
pub struct TrailBuilder {
    trailed_mem: MemoryBuilder,
    stable_mem: MemoryBuilder,
}

impl TrailBuilder {
    /// Creates a new empty `TrailBuilder`.
    pub fn new() -> Self {
        Self {
            trailed_mem: MemoryBuilder::new(),
            stable_mem: MemoryBuilder::new(),
        }
    }

    /// Consumes the `TrailBuilder` to create a new `Trail`.
    pub fn finish(self) -> Trail {
        Trail {
            trailed_mem: self.trailed_mem.finish(),
            stable_mem: self.stable_mem.finish(),
            trail: vec![],
        }
    }
}

/// A reference to a value stored on the trail.
///
/// How the value is stored on the trail depends on the type parameter `M`.
/// A `Value<Trailed, T>` is stored in trailed memory,
/// whereas a `Value<Stable, T>` is stored in stable memory.
pub struct Value<M, T> {
    pointer: Pointer<T>,
    phantom: PhantomData<M>,
}

impl<M, T> Value<M, T>
where
    M: StorageMode,
    T: Bytes,
{
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
    /// use contrail::{Stable, TrailBuilder, Value};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let stable = Value::<Stable, _>::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(stable.get(&trail), 5);
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
    /// use contrail::{Trail, TrailBuilder, TrailedValue};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let trailed = TrailedValue::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// trailed.set(&mut trail, 42);
    /// assert_eq!(trailed.get(&trail), 42);
    /// ```
    #[inline(always)]
    pub fn set(self, trail: &mut Trail, new_val: T) {
        self.pointer.set(M::memory_mut(trail), new_val);
    }

    /// Updates the value on the trail.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::{Trail, TrailBuilder, TrailedValue};
    ///
    /// let mut builder = TrailBuilder::new();
    /// let trailed = TrailedValue::new(&mut builder, 5);
    /// let mut trail = builder.finish();
    ///
    /// trailed.update(&mut trail, |x| x * x);
    /// assert_eq!(trailed.get(&trail), 25);
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

/// A reference to a fixed-length array of values stored on the trail.
///
/// How the value is stored on the trail depends on the type parameter `M`.
/// A `ArrayValue<Trailed, T>` is stored in trailed memory,
/// whereas a `ArrayValue<Stable, T>` is stored in stable memory.
pub struct Array<M, T> {
    pointer: ArrayPointer<T>,
    phantom: PhantomData<M>,
}

impl<M, T> Array<M, T>
where
    M: StorageMode,
    T: Bytes,
{
    pub fn new(builder: &mut TrailBuilder, vals: impl IntoIterator<Item = T>) -> Self {
        Self {
            pointer: ArrayPointer::new(
                M::builder_mut(builder),
                &vals.into_iter().collect::<Vec<_>>(),
            ),
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.pointer.len()
    }

    #[inline(always)]
    pub fn get(&self, trail: &Trail, i: usize) -> T {
        self.pointer.get(M::memory(trail), i)
    }

    #[inline(always)]
    pub fn set(&self, trail: &mut Trail, i: usize, new_val: T) {
        self.pointer.set(M::memory_mut(trail), i, new_val);
    }

    #[inline(always)]
    pub fn update(&self, trail: &mut Trail, i: usize, f: impl FnOnce(T) -> T) {
        self.pointer.update(M::memory_mut(trail), i, f);
    }

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

/// A value stored in trailed memory.
pub type TrailedValue<T> = Value<Trailed, T>;

/// A value stored in stable memory.
pub type StableValue<T> = Value<Stable, T>;

/// A fixed-length array stored in trailed memory.
pub type TrailedArray<T> = Array<Trailed, T>;

/// A fixed-length array stored in stable memory.
pub type StableArray<T> = Array<Stable, T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_and_store_value() {
        let init_val = 5;
        let new_val = 6;

        let mut builder = TrailBuilder::new();
        let trailed = TrailedValue::new(&mut builder, init_val);
        let stored = StableValue::new(&mut builder, init_val);
        let mut trail = builder.finish();

        assert_eq!(trailed.get(&trail), init_val);
        assert_eq!(stored.get(&trail), init_val);

        trail.new_level();

        assert_eq!(trailed.get(&trail), init_val);
        assert_eq!(stored.get(&trail), init_val);

        trailed.set(&mut trail, new_val);
        stored.set(&mut trail, new_val);

        assert_eq!(trailed.get(&trail), new_val);
        assert_eq!(stored.get(&trail), new_val);

        trail.backtrack();

        assert_eq!(trailed.get(&trail), init_val);
        assert_eq!(stored.get(&trail), new_val);
    }

    #[test]
    fn trail_and_store_array() {
        let init_vals = vec![1, 3, 5, 7];
        let new_vals = vec![2, 4, 6, 8];

        let mut builder = TrailBuilder::new();
        let trailed = TrailedArray::new(&mut builder, init_vals.clone());
        let stored = StableArray::new(&mut builder, init_vals.clone());
        let mut trail = builder.finish();

        for i in 0..4 {
            assert_eq!(trailed.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), init_vals[i]);

            trail.new_level();

            assert_eq!(trailed.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), init_vals[i]);

            trailed.set(&mut trail, i, new_vals[i]);
            stored.set(&mut trail, i, new_vals[i]);

            assert_eq!(trailed.get(&trail, i), new_vals[i]);
            assert_eq!(stored.get(&trail, i), new_vals[i]);

            trail.backtrack();

            assert_eq!(trailed.get(&trail, i), init_vals[i]);
            assert_eq!(stored.get(&trail, i), new_vals[i]);
        }
    }
}
