/*
 * This Source Code Form is subject to the terms of the Mozilla Public License,
 * v. 2.0. If a copy of the MPL was not distributed with this file, You can
 * obtain one at http://mozilla.org/MPL/2.0/.
 */

//! Bit sets.

use contrail::{
    storage::{Backtrackable, NonBacktrackable, StorageMode},
    Array, Trail, TrailBuilder,
};

const BLOCK_SIZE: u64 = 64;

/// A bit set stored in backtrackable storage on the trail.
pub type BacktrackableBitSet = BitSet<Backtrackable>;
/// A bit set stored in non-backtrackable storage on the trail.
pub type NonBacktrackableBitSet = BitSet<NonBacktrackable>;

/// A bit set.
#[derive(Clone, Copy)]
pub struct BitSet<M> {
    blocks: Array<M, u64>,
    max: u64,
}

impl<M> BitSet<M>
where
    M: StorageMode,
{
    pub fn new_full(builder: &mut TrailBuilder, len: u64) -> Self {
        assert!(len > 0);
        let max = len - 1;
        let num_blocks = max / BLOCK_SIZE + 1;
        let blocks = Array::new(builder, vec![!0; num_blocks as usize]);
        Self { blocks, max }
    }

    pub fn new_empty(builder: &mut TrailBuilder, len: u64) -> Self {
        assert!(len > 0);
        let max = len - 1;
        let num_blocks = max / BLOCK_SIZE + 1;
        let blocks = Array::new(builder, vec![0; num_blocks as usize]);
        Self { blocks, max }
    }

    pub fn clear(&self, trail: &mut Trail) {
        for i in 0..self.blocks.len() {
            self.blocks.set(trail, i, 0);
        }
    }

    pub fn capacity(&self) -> u64 {
        self.max + 1
    }

    pub fn insert(&self, trail: &mut Trail, value: u64) {
        if value <= self.max {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            self.blocks
                .set(trail, index, block | (1 << (value % BLOCK_SIZE)));
        }
    }

    pub fn contains(&self, trail: &Trail, value: u64) -> bool {
        if value > self.max {
            false
        } else {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            (block >> (value % BLOCK_SIZE)) & 1 == 1
        }
    }

    pub fn remove(&self, trail: &mut Trail, value: u64) {
        if value <= self.max {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            self.blocks
                .set(trail, index, block & !(1 << (value % BLOCK_SIZE)));
        }
    }

    pub fn count_between(&self, trail: &Trail, min: u64, max: u64) -> u64 {
        if min <= max && min <= self.max {
            let max = max.min(self.max);
            let min_block_index = (min / BLOCK_SIZE) as usize;
            let max_block_index = (max / BLOCK_SIZE) as usize;
            let min_offset = min % BLOCK_SIZE;
            let max_offset = max % BLOCK_SIZE;
            let min_mask = !0 << min_offset;
            let max_mask = !0 >> (BLOCK_SIZE - max_offset - 1);
            if min_block_index == max_block_index {
                let mask = min_mask & max_mask;
                let block = self.blocks.get(trail, min_block_index);
                u64::from((block & mask).count_ones())
            } else {
                let min_block = self.blocks.get(trail, min_block_index);
                let min_block_count = u64::from((min_block & min_mask).count_ones());
                let max_block = self.blocks.get(trail, max_block_index);
                let max_block_count = u64::from((max_block & max_mask).count_ones());
                let mut tot = min_block_count + max_block_count;
                for i in (min_block_index + 1)..max_block_index {
                    let block = self.blocks.get(trail, i as usize);
                    tot += u64::from(block.count_ones());
                }
                tot
            }
        } else {
            0
        }
    }

    pub fn next_above(&self, trail: &Trail, value: u64) -> Option<u64> {
        if value > self.max {
            None
        } else {
            let block = value / BLOCK_SIZE;
            let offset = value % BLOCK_SIZE;
            let to_skip =
                (self.blocks.get(trail, block as usize) >> offset).trailing_zeros() as u64;
            if to_skip == BLOCK_SIZE {
                self.next_above(trail, (block + 1) * BLOCK_SIZE)
            } else if value + to_skip > self.max {
                None
            } else {
                Some((value + to_skip) as u64)
            }
        }
    }

    pub fn next_below(&self, trail: &Trail, value: u64) -> Option<u64> {
        let value = value.min(self.max);
        let block = value / BLOCK_SIZE;
        let offset = value % BLOCK_SIZE;
        let to_skip = (self.blocks.get(trail, block as usize) << (BLOCK_SIZE - offset - 1))
            .leading_zeros() as u64;
        if to_skip == BLOCK_SIZE {
            if block == 0 {
                None
            } else {
                self.next_below(trail, block * BLOCK_SIZE - 1)
            }
        } else {
            Some((value - to_skip) as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let mut builder = TrailBuilder::new();
        let empty = BacktrackableBitSet::new_empty(&mut builder, 10);
        let full = BacktrackableBitSet::new_full(&mut builder, 10);
        let trail = builder.finish();
        
        assert_eq!(empty.capacity(), 10);
        assert_eq!(full.capacity(), 10);

        for i in 0..10 {
            assert!(!empty.contains(&trail, i));
            assert!(full.contains(&trail, i));
        }
    }

    #[test]
    fn clear() {
        let mut builder = TrailBuilder::new();
        let bit_set = BacktrackableBitSet::new_full(&mut builder, 100);
        let mut trail = builder.finish();

        for i in 0..100 {
            assert!(bit_set.contains(&trail, i));
        }

        bit_set.clear(&mut trail);

        for i in 0..100 {
            assert!(!bit_set.contains(&trail, i));
        }
    }

    #[test]
    fn insert_remove_contains() {
        let mut builder = TrailBuilder::new();
        let bit_set = BacktrackableBitSet::new_empty(&mut builder, 10);
        let mut trail = builder.finish();

        assert!(!bit_set.contains(&trail, 100));
        
        for i in 0..10 {
            assert!(!bit_set.contains(&trail, i));
            bit_set.insert(&mut trail, i);
            assert!(bit_set.contains(&trail, i));
            bit_set.remove(&mut trail, i);
            assert!(!bit_set.contains(&trail, i));
        }
    }

    #[test]
    fn count_between() {
        let mut builder = TrailBuilder::new();
        let small = BacktrackableBitSet::new_full(&mut builder, 10);
        let medium = BacktrackableBitSet::new_full(&mut builder, 100);
        let large = BacktrackableBitSet::new_full(&mut builder, 1000);
        let trail = builder.finish();

        assert_eq!(small.count_between(&trail, 0, 9), 10);
        assert_eq!(small.count_between(&trail, 1, 8), 8);
        assert_eq!(small.count_between(&trail, 5, 5), 1);
        assert_eq!(small.count_between(&trail, 6, 5), 0);

        assert_eq!(medium.count_between(&trail, 0, 99), 100);
        assert_eq!(medium.count_between(&trail, 1, 98), 98);
        assert_eq!(medium.count_between(&trail, 50, 50), 1);
        assert_eq!(medium.count_between(&trail, 51, 50), 0);

        assert_eq!(large.count_between(&trail, 0, 999), 1000);
        assert_eq!(large.count_between(&trail, 1, 998), 998);
        assert_eq!(large.count_between(&trail, 500, 500), 1);
        assert_eq!(large.count_between(&trail, 501, 500), 0);
    }

    #[test]
    fn next_above_below() {
        let mut builder = TrailBuilder::new();
        let bit_set = BacktrackableBitSet::new_empty(&mut builder, 1000);
        let mut trail = builder.finish();

        for i in 1..10 {
            bit_set.insert(&mut trail, i * 100);
        }

        for i in 0..100 {
            assert_eq!(bit_set.next_below(&trail, i), None);
            assert_eq!(bit_set.next_above(&trail, i), Some(100));
        }

        for i in 401..500 {
            assert_eq!(bit_set.next_below(&trail, i), Some(400));
            assert_eq!(bit_set.next_above(&trail, i), Some(500));
        }

        for i in 901..1000 {
            assert_eq!(bit_set.next_below(&trail, i), Some(900));
            assert_eq!(bit_set.next_above(&trail, i), None);
        }
    }
}
