/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Sparse sets.

use contrail::{
    storage::{Backtrackable, NonBacktrackable, StorageMode},
    NonBacktrackableArray, Trail, TrailBuilder, Value,
};
use std::fmt;

/// A sparse set stored on the trail in backtrackable memory.
pub type BacktrackableSparseSet = SparseSet<Backtrackable>;

/// A sparse set stored on the trail in non-backtrackable memory.
pub type NonBacktrackableSparseSet = SparseSet<NonBacktrackable>;

/// A specialized backtrackable data structure for storing subsets of the range `0..n` that can
/// only decrease in size.
///
/// Features O(1) `contains()` and `remove()` as well as fast value iteration.
pub struct SparseSet<M> {
    values: NonBacktrackableArray<usize>,
    positions: NonBacktrackableArray<usize>,
    len: Value<M, usize>,
}

impl<M> SparseSet<M>
where
    M: StorageMode,
{
    /// Creates a new `SparseSet` initialized with the values `0..len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(sparse_set.len(&trail), 10);
    /// ```
    pub fn new_full(builder: &mut TrailBuilder, len: usize) -> Self {
        Self {
            values: NonBacktrackableArray::new(builder, 0..len),
            positions: NonBacktrackableArray::new(builder, 0..len),
            len: Value::new(builder, len),
        }
    }

    /// Returns an iterator over the elements of the `SparseSet` in arbitrary order.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// // remove some values
    /// sparse_set.remove(&mut trail, 2);
    /// sparse_set.remove(&mut trail, 3);
    /// sparse_set.remove(&mut trail, 6);
    ///
    /// for value in sparse_set.iter(&trail) {
    ///     assert!(value != 2 && value != 3 && value != 6);
    /// }
    /// ```
    pub fn iter<'s, 't: 's>(&'s self, trail: &'t Trail) -> impl Iterator<Item = usize> + 's {
        (0..self.len.get(trail)).map(move |i| self.values.get(trail, i))
    }

    /// Returns the length of the `SparseSet`.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// assert_eq!(sparse_set.len(&trail), 10);
    ///
    /// sparse_set.remove(&mut trail, 5);
    ///
    /// assert_eq!(sparse_set.len(&trail), 9);
    /// ```
    pub fn len(&self, trail: &Trail) -> usize {
        self.len.get(trail)
    }

    /// Returns true if the sparse set is empty and false otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 1);
    /// let mut trail = builder.finish();
    ///
    /// assert!(!sparse_set.is_empty(&trail));
    ///
    /// sparse_set.remove(&mut trail, 0);
    ///
    /// assert!(sparse_set.is_empty(&trail));
    /// ```
    pub fn is_empty(&self, trail: &Trail) -> bool {
        self.len.get(trail) == 0
    }

    /// Checks if the sparse set contains the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// assert!(sparse_set.contains(&trail, 5));
    /// assert!(!sparse_set.contains(&trail, 15));
    /// ```
    pub fn contains(&self, trail: &Trail, val: usize) -> bool {
        val < self.positions.len() && self.positions.get(trail, val) < self.len.get(trail)
    }

    /// Removes a value from the sparse set.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// assert!(sparse_set.contains(&trail, 5));
    /// assert_eq!(sparse_set.len(&trail), 10);
    ///
    /// sparse_set.remove(&mut trail, 5);
    ///
    /// assert!(!sparse_set.contains(&trail, 5));
    /// assert_eq!(sparse_set.len(&trail), 9);
    /// ```
    pub fn remove(&self, trail: &mut Trail, val: usize) {
        if self.contains(trail, val) {
            let position = self.positions.get(trail, val);
            let new_size = self.len.get(trail) - 1;
            self.swap(trail, position, new_size);
            self.len.set(trail, new_size);
        }
    }

    /// Filters the elements in the sparse set according to the predicate
    /// function.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// // create a sparse set initialized with the values 0..10
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// // keep only the odd numbers
    /// sparse_set.filter(&mut trail, |x| x % 2 == 1);
    ///
    /// // make sure we kept only the odd numbers
    /// let mut values = sparse_set.iter(&trail).collect::<Vec<_>>();
    /// // we have to sort the values because a sparse set is unordered
    /// values.sort();
    /// assert_eq!(values, vec![1, 3, 5, 7, 9]);
    /// ```
    pub fn filter(&self, trail: &mut Trail, f: impl Fn(usize) -> bool) {
        for position in (0..self.len.get(trail)).rev() {
            let val = self.values.get(trail, position);
            if !f(val) {
                let new_size = self.len.get(trail) - 1;
                self.swap(trail, position, new_size);
                self.len.set(trail, new_size);
            }
        }
    }

    /// Intersects the sparse set with the given values.
    ///
    /// # Examples
    ///
    /// ```
    /// use contrail::TrailBuilder;
    /// use contrail_collections::sparse_set::BacktrackableSparseSet;
    ///
    /// // create a sparse set initialized with the elements 0..10
    /// let mut builder = TrailBuilder::new();
    /// let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
    /// let mut trail = builder.finish();
    ///
    /// // keep only the fibonacci numbers
    /// sparse_set.intersect(&mut trail, vec![0, 1, 1, 2, 3, 5, 8, 13]);
    ///
    /// // make sure we kept only the fibonacci numbers
    /// let mut values = sparse_set.iter(&trail).collect::<Vec<_>>();
    /// // we have to sort the values because a sparse set is unordered
    /// values.sort();
    /// assert_eq!(values, vec![0, 1, 2, 3, 5, 8]);
    /// ```
    pub fn intersect(&self, trail: &mut Trail, vals: impl IntoIterator<Item = usize>) {
        let mut vals = vals.into_iter().collect::<Vec<_>>();
        vals.sort();
        vals.dedup();
        let mut new_size = 0;
        for val in vals {
            if self.contains(trail, val) {
                let position = self.positions.get(trail, val);
                self.swap(trail, position, new_size);
                new_size += 1;
            }
        }
        self.len.set(trail, new_size);
    }

    /// Swaps two positions in the sparse set.
    fn swap(&self, trail: &mut Trail, i: usize, j: usize) {
        let val_i = self.values.get(trail, i);
        let val_j = self.values.get(trail, j);

        self.values.set(trail, i, val_j);
        self.values.set(trail, j, val_i);

        self.positions.set(trail, val_i, j);
        self.positions.set(trail, val_j, i);
    }
}

impl<M> Clone for SparseSet<M> {
    fn clone(&self) -> Self {
        Self {
            values: self.values,
            positions: self.positions,
            len: self.len,
        }
    }
}

impl<M> Copy for SparseSet<M> {}

impl<M> fmt::Debug for SparseSet<M>
where
    M: StorageMode,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SparseSet")
            .field("values", &self.values)
            .field("positions", &self.positions)
            .field("len", &self.len)
            .finish()
    }
}

impl<M> Eq for SparseSet<M> {}

impl<M> PartialEq for SparseSet<M> {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values && self.positions == other.positions && self.len == other.len
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use contrail::TrailBuilder;

    #[test]
    fn clone_eq() {
        let mut builder = TrailBuilder::new();
        let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);

        let clone = sparse_set.clone();
        assert_eq!(sparse_set, clone);
    }

    #[test]
    fn debug() {
        let mut builder = TrailBuilder::new();
        let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
        assert_eq!(format!("{:?}", sparse_set), "SparseSet { values: Array { pointer: ArrayPointer { offset: 0, len: 10 }, storage_mode: NonBacktrackable }, positions: Array { pointer: ArrayPointer { offset: 80, len: 10 }, storage_mode: NonBacktrackable }, len: Value { pointer: Pointer { offset: 0 }, storage_mode: Backtrackable } }");
    }

    #[test]
    fn iter() {
        let mut builder = TrailBuilder::new();
        let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 6);
        let mut trail = builder.finish();

        sparse_set.remove(&mut trail, 1);
        sparse_set.remove(&mut trail, 3);
        sparse_set.remove(&mut trail, 5);

        let mut values = sparse_set.iter(&trail).collect::<Vec<_>>();
        values.sort();
        assert_eq!(values, &[0, 2, 4]);
    }

    #[test]
    fn is_empty() {
        let mut builder = TrailBuilder::new();
        let empty = BacktrackableSparseSet::new_full(&mut builder, 0);
        let not_empty = BacktrackableSparseSet::new_full(&mut builder, 1);
        let trail = builder.finish();

        assert_eq!(empty.len(&trail), 0);
        assert!(empty.is_empty(&trail));

        assert_eq!(not_empty.len(&trail), 1);
        assert!(!not_empty.is_empty(&trail));
    }

    #[test]
    fn filter() {
        let mut builder = TrailBuilder::new();
        let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
        let mut trail = builder.finish();

        sparse_set.filter(&mut trail, |x| x % 2 == 1);

        let mut values = sparse_set.iter(&trail).collect::<Vec<_>>();
        values.sort();
        assert_eq!(values, vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn intersect() {
        let mut builder = TrailBuilder::new();
        let sparse_set = BacktrackableSparseSet::new_full(&mut builder, 10);
        let mut trail = builder.finish();

        sparse_set.intersect(&mut trail, vec![0, 1, 1, 2, 3, 5, 8, 13]);

        let mut values = sparse_set.iter(&trail).collect::<Vec<_>>();
        values.sort();
        assert_eq!(values, vec![0, 1, 2, 3, 5, 8]);
    }

    #[test]
    fn backtrack() {
        let mut builder = TrailBuilder::new();

        // 0..5
        let trailed_sparse_set = BacktrackableSparseSet::new_full(&mut builder, 5);

        let trail = &mut builder.finish();

        trail.new_level();

        trailed_sparse_set.remove(trail, 1);
        assert!(!trailed_sparse_set.contains(trail, 1));

        trail.new_level();

        trailed_sparse_set.remove(trail, 4);
        assert!(!trailed_sparse_set.contains(trail, 4));

        trailed_sparse_set.remove(trail, 2);

        assert!(!trailed_sparse_set.contains(trail, 4));

        trail.new_level();

        trailed_sparse_set.remove(trail, 0);
        assert!(!trailed_sparse_set.contains(trail, 0));

        trail.backtrack();

        assert!(trailed_sparse_set.contains(trail, 0));

        trail.backtrack();

        assert!(trailed_sparse_set.contains(trail, 4));
        assert!(trailed_sparse_set.contains(trail, 2));

        trail.backtrack();

        assert!(trailed_sparse_set.contains(trail, 1));
    }
}
