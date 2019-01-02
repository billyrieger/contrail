//! High-level memory management.
use crate::{Memory, MemoryBuilder, Trail, TrailBuilder};

/// Representation of how something is stored on the trail.
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
