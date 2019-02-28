/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

use contrail::{
    storage::{Backtrackable, NonBacktrackable, StorageMode},
    NonBacktrackableArray, Trail, TrailBuilder, Value,
};

pub type BacktrackableSparseSet = SparseSet<Backtrackable>;
pub type NonBacktrackableSparseSet = SparseSet<NonBacktrackable>;

/// A specialized backtrackable data structure for storing subsets of the range `0..n` that can
/// only decrease in size.
///
/// Features O(1) `contains` and `remove` as well as fast value iteration.
#[derive(Clone, Copy, Debug)]
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
    pub fn is_empty(&self, trail: &Trail) -> bool {
        self.len.get(trail) == 0
    }

    pub fn contains(&self, trail: &Trail, val: usize) -> bool {
        val < self.positions.len() && self.positions.get(trail, val) < self.len.get(trail)
    }

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

    fn swap(&self, trail: &mut Trail, i: usize, j: usize) {
        let val_i = self.values.get(trail, i);
        let val_j = self.values.get(trail, j);

        self.values.set(trail, i, val_j);
        self.values.set(trail, j, val_i);

        self.positions.set(trail, val_i, j);
        self.positions.set(trail, val_j, i);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use contrail::TrailBuilder;

    #[test]
    fn basic() {
        let mut builder = TrailBuilder::new();

        let set = BacktrackableSparseSet::new_full(&mut builder, 10);
        let mut trail = builder.finish();

        trail.new_level();

        assert_eq!(set.len(&trail), 10);
    }

    #[test]
    fn test() {
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
