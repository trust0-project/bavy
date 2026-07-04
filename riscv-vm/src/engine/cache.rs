//! Block Cache for the JIT-less Superblock Engine.
//!
//! Direct-mapped open-addressed array of compiled blocks keyed by PC.
//! This replaces the previous HashMap<u64, Box<Block>>: block lookup is the
//! hottest dispatch operation in the VM and the hash + probe + Box pointer
//! chase cost ~2.5x a TLB hit. A direct-mapped array is one index + one tag
//! compare on a linear slab.
//!
//! Invalidation is generation-based: bumping `generation` makes every cached
//! block stale without touching the slab (stale entries fail the tag check).

use super::block::Block;
#[cfg(test)]
use super::microop::MicroOp;

/// Number of cache slots (power of two for mask indexing).
pub const BLOCK_CACHE_SIZE: usize = 4096;
const BLOCK_CACHE_MASK: u64 = (BLOCK_CACHE_SIZE as u64) - 1;

/// One direct-mapped slot. `valid` + the block's own start_pc form the tag.
struct Slot {
    valid: bool,
    block: Block,
}

/// Direct-mapped block cache indexed by PC.
pub struct BlockCache {
    /// Boxed slab: ~BLOCK_CACHE_SIZE x sizeof(Block); heap-allocated so Cpu
    /// stays cheap to construct and move.
    slots: Box<[Slot]>,
    /// Current generation (incremented on flush).
    pub generation: u32,
    /// Statistics: cache hits.
    pub hits: u64,
    /// Statistics: cache misses.
    pub misses: u64,
    /// Statistics: invalidations.
    pub invalidations: u64,
}

impl BlockCache {
    /// Create a new empty block cache.
    pub fn new() -> Self {
        let slots = (0..BLOCK_CACHE_SIZE)
            .map(|_| Slot {
                valid: false,
                block: Block::new(0, 0, 0),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            slots,
            generation: 0,
            hits: 0,
            misses: 0,
            invalidations: 0,
        }
    }

    /// Slot index for a PC. Instructions are 2-byte aligned (C extension),
    /// so shift out the low bit before masking.
    #[inline(always)]
    fn index(pc: u64) -> usize {
        ((pc >> 1) & BLOCK_CACHE_MASK) as usize
    }

    /// Look up a block by PC.
    #[inline]
    pub fn get(&mut self, pc: u64) -> Option<&Block> {
        // SAFETY: index() is always < BLOCK_CACHE_SIZE due to the mask
        let slot = unsafe { self.slots.get_unchecked(Self::index(pc)) };
        if slot.valid && slot.block.start_pc == pc && slot.block.generation == self.generation {
            self.hits += 1;
            Some(&slot.block)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Look up a block and increment its exec_count in a single operation.
    #[inline(always)]
    pub fn get_and_touch(&mut self, pc: u64) -> Option<&Block> {
        // SAFETY: index() is always < BLOCK_CACHE_SIZE due to the mask
        let slot = unsafe { self.slots.get_unchecked_mut(Self::index(pc)) };
        if slot.valid && slot.block.start_pc == pc && slot.block.generation == self.generation {
            self.hits += 1;
            slot.block.exec_count = slot.block.exec_count.saturating_add(1);
            Some(&slot.block)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Get mutable block for updating exec_count.
    #[inline]
    pub fn get_mut(&mut self, pc: u64) -> Option<&mut Block> {
        let generation = self.generation;
        let slot = &mut self.slots[Self::index(pc)];
        if slot.valid && slot.block.start_pc == pc && slot.block.generation == generation {
            Some(&mut slot.block)
        } else {
            None
        }
    }

    /// Insert a compiled block, evicting whatever occupied its slot.
    pub fn insert(&mut self, block: Block) {
        let slot = &mut self.slots[Self::index(block.start_pc)];
        slot.block = block;
        slot.valid = true;
    }

    /// Invalidate all blocks (called on SATP change, SFENCE.VMA).
    pub fn flush(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.invalidations += 1;
        // Slots are not cleared; stale entries fail the generation check.
    }

    /// Invalidate blocks in a specific physical address range.
    /// Called when code is modified.
    pub fn invalidate_range(&mut self, start_pa: u64, end_pa: u64) {
        for slot in self.slots.iter_mut() {
            if slot.valid {
                let block_end = slot.block.start_pa + slot.block.byte_len as u64;
                if slot.block.start_pa < end_pa && block_end > start_pa {
                    slot.valid = false;
                }
            }
        }
        self.invalidations += 1;
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.valid = false;
        }
        self.generation = 0;
        self.hits = 0;
        self.misses = 0;
        self.invalidations = 0;
    }

    /// Number of valid entries (O(n); diagnostics only).
    fn valid_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.valid && s.block.generation == self.generation)
            .count()
    }

    /// Get cache statistics as a tuple: (hits, misses, size, hit_rate).
    pub fn stats(&self) -> (u64, u64, usize, f64) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
        (self.hits, self.misses, self.valid_count(), hit_rate)
    }
}

impl Default for BlockCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_block(pc: u64, generation: u32) -> Block {
        let mut block = Block::new(pc, pc, generation);
        block.push(
            MicroOp::Addi {
                rd: 1,
                rs1: 0,
                imm: 1,
            },
            4,
        );
        block
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = BlockCache::new();
        let block = make_test_block(0x8000_0000, cache.generation);
        cache.insert(block);

        let found = cache.get(0x8000_0000);
        assert!(found.is_some());
        assert_eq!(cache.hits, 1);
        assert_eq!(cache.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = BlockCache::new();
        let found = cache.get(0x8000_0000);
        assert!(found.is_none());
        assert_eq!(cache.hits, 0);
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn test_cache_flush_invalidates() {
        let mut cache = BlockCache::new();
        let block = make_test_block(0x8000_0000, cache.generation);
        cache.insert(block);

        // Block should be found before flush
        assert!(cache.get(0x8000_0000).is_some());

        // Flush and try again
        cache.flush();
        assert!(cache.get(0x8000_0000).is_none());
        assert_eq!(cache.invalidations, 1);
    }

    #[test]
    fn test_cache_generation_check() {
        let mut cache = BlockCache::new();

        // Insert block with old generation
        let block = make_test_block(0x8000_0000, 0);
        cache.generation = 1; // Advance generation past the block's
        cache.insert(block);

        // Should not find it due to generation mismatch
        assert!(cache.get(0x8000_0000).is_none());
    }

    #[test]
    fn test_cache_direct_mapped_eviction() {
        let mut cache = BlockCache::new();
        let pc_a = 0x8000_0000u64;
        // Same slot: differs by exactly BLOCK_CACHE_SIZE instruction slots.
        let pc_b = pc_a + (BLOCK_CACHE_SIZE as u64) * 2;
        cache.insert(make_test_block(pc_a, 0));
        assert!(cache.get(pc_a).is_some());
        cache.insert(make_test_block(pc_b, 0));
        // pc_b evicted pc_a (same slot), pc_b hits.
        assert!(cache.get(pc_b).is_some());
        assert!(cache.get(pc_a).is_none());
    }

    #[test]
    fn test_cache_invalidate_range() {
        let mut cache = BlockCache::new();
        cache.insert(make_test_block(0x8000_0000, 0));
        cache.insert(make_test_block(0x8000_1000, 0));
        // Invalidate only the first page.
        cache.invalidate_range(0x8000_0000, 0x8000_0800);
        assert!(cache.get(0x8000_0000).is_none());
        assert!(cache.get(0x8000_1000).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = BlockCache::new();
        let block = make_test_block(0x8000_0000, cache.generation);
        cache.insert(block);

        // Hit
        cache.get(0x8000_0000);
        // Miss
        cache.get(0x8000_1000);
        cache.get(0x8000_2000);

        let (hits, misses, size, hit_rate) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 2);
        assert_eq!(size, 1);
        assert!((hit_rate - 0.333).abs() < 0.01);
    }
}
