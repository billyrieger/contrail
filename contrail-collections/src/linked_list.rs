/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

use contrail::{
    mem::Bytes,
    storage::{Backtrackable, NonBacktrackable, StorageMode},
    Array, Trail, TrailBuilder,
};
use std::fmt;

pub type BacktrackableLinkedListArena<T> = LinkedListArena<Backtrackable, T>;
pub type NonBacktrackableLinkedListArena<T> = LinkedListArena<NonBacktrackable, T>;
pub type BacktrackableLinkedListNode<T> = LinkedListNode<Backtrackable, T>;
pub type NonBacktrackableLinkedListNode<T> = LinkedListNode<NonBacktrackable, T>;

pub struct LinkedListNode<M, T> {
    prev: Array<M, usize>,
    next: Array<M, usize>,
    data: Array<M, T>,
    index: usize,
}

impl<M, T> LinkedListNode<M, T>
where
    M: StorageMode,
    T: Bytes,
{
    /// Gets the data associated with this node from the trail.
    pub fn data(&self, trail: &Trail) -> T {
        self.data.get(trail, self.index)
    }

    pub fn set_data(&self, trail: &mut Trail, new_data: T) {
        self.data.set(trail, self.index, new_data);
    }

    pub fn next(&self, trail: &Trail) -> LinkedListNode<M, T> {
        LinkedListNode {
            prev: self.prev,
            next: self.next,
            data: self.data,
            index: self.next.get(trail, self.index),
        }
    }

    pub fn prev(&self, trail: &Trail) -> LinkedListNode<M, T> {
        LinkedListNode {
            prev: self.prev,
            next: self.next,
            data: self.data,
            index: self.prev.get(trail, self.index),
        }
    }

    pub fn unlink(&self, trail: &mut Trail) {
        // unlink self from current list
        self.prev(trail).set_next(trail, self.next(trail));
        self.next(trail).set_prev(trail, self.prev(trail));

        // link self to itself
        self.set_prev(trail, *self);
        self.set_next(trail, *self);
    }

    pub fn insert_after(&self, trail: &mut Trail, node: LinkedListNode<M, T>) {
        // unlink self from current list
        self.prev(trail).set_next(trail, self.next(trail));
        self.next(trail).set_prev(trail, self.prev(trail));

        // add self before node.next(trail)
        let next = node.next(trail);
        self.set_next(trail, next);
        next.set_prev(trail, *self);

        // add node before self
        node.set_next(trail, *self);
        self.set_prev(trail, node);
    }

    pub fn insert_before(&self, trail: &mut Trail, node: LinkedListNode<M, T>) {
        // unlink self from current list
        self.prev(trail).set_next(trail, self.next(trail));
        self.next(trail).set_prev(trail, self.prev(trail));

        // add node.prev(trail) before self
        let prev = node.prev(trail);
        prev.set_next(trail, *self);
        self.set_prev(trail, prev);

        // add self before node
        self.set_next(trail, node);
        node.set_prev(trail, *self);
    }

    fn set_next(&self, trail: &mut Trail, next: LinkedListNode<M, T>) {
        self.next.set(trail, self.index, next.index);
    }

    fn set_prev(&self, trail: &mut Trail, prev: LinkedListNode<M, T>) {
        self.prev.set(trail, self.index, prev.index);
    }
}

impl<M, T> Clone for LinkedListNode<M, T> {
    fn clone(&self) -> Self {
        Self {
            prev: self.prev,
            next: self.next,
            data: self.data,
            index: self.index,
        }
    }
}

impl<M, T> Copy for LinkedListNode<M, T> {}

impl<M, T> fmt::Debug for LinkedListNode<M, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LinkedListNode")
            .field("index", &self.index)
            .finish()
    }
}

impl<M, T> Eq for LinkedListNode<M, T> {}

impl<M, T> PartialEq for LinkedListNode<M, T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

pub struct LinkedListArena<M, T> {
    prev: Array<M, usize>,
    next: Array<M, usize>,
    data: Array<M, T>,
}

impl<M, T> LinkedListArena<M, T>
where
    M: StorageMode,
    T: Bytes,
{
    pub fn new(builder: &mut TrailBuilder, data: Vec<T>) -> Self {
        Self {
            prev: Array::new(builder, 0..data.len()),
            next: Array::new(builder, 0..data.len()),
            data: Array::new(builder, data),
        }
    }

    pub fn node(&self, i: usize) -> LinkedListNode<M, T> {
        assert!(i < self.data.len(), "node index out of bounds");
        LinkedListNode {
            prev: self.prev,
            next: self.next,
            data: self.data,
            index: i,
        }
    }
}

impl<M, T> Clone for LinkedListArena<M, T> {
    fn clone(&self) -> Self {
        Self {
            prev: self.prev,
            next: self.next,
            data: self.data,
        }
    }
}

impl<M, T> Copy for LinkedListArena<M, T> {}

impl<M, T> fmt::Debug for LinkedListArena<M, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LinkedListArena").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod node {
        use super::*;

        #[test]
        fn debug() {
            let mut builder = TrailBuilder::new();
            let arena =
                NonBacktrackableLinkedListArena::new(&mut builder, (0..2).map(|_| ()).collect());

            let node0 = arena.node(0);
            let node1 = arena.node(1);

            assert_eq!(format!("{:?}", node0), "LinkedListNode { index: 0 }");
            assert_eq!(format!("{:?}", node1), "LinkedListNode { index: 1 }");
        }

        #[test]
        fn clone_eq() {
            let mut builder = TrailBuilder::new();
            let arena =
                NonBacktrackableLinkedListArena::new(&mut builder, (0..2).map(|_| ()).collect());

            let node = arena.node(0);
            let node_clone = node.clone();
            assert_eq!(node, node_clone);
        }
    }

    mod arena {
        use super::*;

        #[test]
        fn debug() {
            let mut builder = TrailBuilder::new();
            let arena =
                NonBacktrackableLinkedListArena::new(&mut builder, (0..2).map(|_| ()).collect());

            assert_eq!(format!("{:?}", arena), "LinkedListArena");
        }

        #[test]
        fn clone() {
            let mut builder = TrailBuilder::new();
            let arena =
                NonBacktrackableLinkedListArena::new(&mut builder, (0..2).map(|_| ()).collect());

            let arena_clone = arena.clone();

            assert_eq!(arena.node(1), arena_clone.node(1));
        }

    }

    #[test]
    fn get_set_data() {
        let mut builder = TrailBuilder::new();
        let arena = BacktrackableLinkedListArena::new(&mut builder, (10..15).collect());
        let mut trail = builder.finish();

        let node = arena.node(3);
        assert_eq!(node.data(&trail), 13);

        node.set_data(&mut trail, 23);
        assert_eq!(node.data(&trail), 23);
    }

    #[test]
    fn unlink() {
        let mut builder = TrailBuilder::new();
        let arena =
            NonBacktrackableLinkedListArena::new(&mut builder, (0..2).map(|_| ()).collect());
        let mut trail = builder.finish();

        let a = arena.node(0);
        let b = arena.node(1);

        a.insert_before(&mut trail, b);

        assert_eq!(a.next(&trail).index, b.index);
        assert_eq!(a.prev(&trail).index, b.index);
        assert_eq!(b.next(&trail).index, a.index);
        assert_eq!(b.prev(&trail).index, a.index);

        b.unlink(&mut trail);

        assert_eq!(a.next(&trail).index, a.index);
        assert_eq!(a.prev(&trail).index, a.index);
        assert_eq!(b.next(&trail).index, b.index);
        assert_eq!(b.prev(&trail).index, b.index);
    }

    #[test]
    fn insert_after() {
        let mut builder = TrailBuilder::new();
        let arena =
            NonBacktrackableLinkedListArena::new(&mut builder, (0..3).map(|_| ()).collect());
        let mut trail = builder.finish();

        let a = arena.node(0);
        let b = arena.node(1);
        let c = arena.node(2);

        b.insert_after(&mut trail, a);
        c.insert_after(&mut trail, b);

        assert_eq!(a.next(&trail).index, b.index);
        assert_eq!(b.next(&trail).index, c.index);
        assert_eq!(c.next(&trail).index, a.index);

        assert_eq!(a.prev(&trail).index, c.index);
        assert_eq!(c.prev(&trail).index, b.index);
        assert_eq!(b.prev(&trail).index, a.index);
    }

    #[test]
    fn insert_before() {
        let mut builder = TrailBuilder::new();
        let arena =
            NonBacktrackableLinkedListArena::new(&mut builder, (0..3).map(|_| ()).collect());
        let mut trail = builder.finish();

        let a = arena.node(0);
        let b = arena.node(1);
        let c = arena.node(2);

        b.insert_before(&mut trail, a);
        c.insert_before(&mut trail, b);

        assert_eq!(c.next(&trail).index, b.index);
        assert_eq!(b.next(&trail).index, a.index);
        assert_eq!(a.next(&trail).index, c.index);

        assert_eq!(c.prev(&trail).index, a.index);
        assert_eq!(a.prev(&trail).index, b.index);
        assert_eq!(b.prev(&trail).index, c.index);
    }
}
