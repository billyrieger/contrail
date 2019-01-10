use contrail::{
    storage::{Stable, StorageMode, Trailed},
    Array, Trail, TrailBuilder,
};

const BLOCK_SIZE: u64 = 64;

#[derive(Clone, Copy)]
pub struct BitSet<M> {
    blocks: Array<M, u64>,
    max: u64,
}

pub type TrailedBitSet = BitSet<Trailed>;
pub type StableBitSet = BitSet<Stable>;

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

    #[inline(always)]
    pub fn clear(&self, trail: &mut Trail) {
        for i in 0..self.blocks.len() {
            self.blocks.set(trail, i, 0);
        }
    }

    #[inline(always)]
    pub fn len(&self) -> u64 {
        self.max + 1
    }

    #[inline(always)]
    pub fn insert(&self, trail: &mut Trail, value: u64) {
        if value <= self.max {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            self.blocks
                .set(trail, index, block | (1 << (value % BLOCK_SIZE)));
        }
    }

    #[inline(always)]
    pub fn contains(&self, trail: &Trail, value: u64) -> bool {
        if value > self.max {
            false
        } else {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            (block >> (value % BLOCK_SIZE)) & 1 == 1
        }
    }

    #[inline(always)]
    pub fn remove(&self, trail: &mut Trail, value: u64) {
        if value <= self.max {
            let index = (value / BLOCK_SIZE) as usize;
            let block = self.blocks.get(trail, index);
            self.blocks
                .set(trail, index, block & !(1 << (value % BLOCK_SIZE)));
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn next_above(&self, trail: &Trail, value: u64) -> Option<u64> {
        if value > self.max {
            None
        } else {
            let block = value / BLOCK_SIZE;
            let offset = value % BLOCK_SIZE;
            let to_skip = (self.blocks.get(trail, block as usize) >> offset).trailing_zeros() as u64;
            if to_skip == BLOCK_SIZE {
                self.next_above(trail, (block + 1) * BLOCK_SIZE)
            } else if value + to_skip > self.max {
                None
            } else {
                Some((value + to_skip) as u64)
            }
        }
    }

    #[inline(always)]
    pub fn next_below(&self, trail: &Trail, value: u64) -> Option<u64> {
        let value = value.min(self.max);
        let block = value / BLOCK_SIZE;
        let offset = value % BLOCK_SIZE;
        let to_skip =
            (self.blocks.get(trail, block as usize) << (BLOCK_SIZE - offset - 1)).leading_zeros() as u64;
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
