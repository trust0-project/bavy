//! Block Cache for the JIT-less Superblock Engine.
//!
//! Manages a cache of compiled basic blocks keyed by PC. Uses generation-based
//! invalidation for efficient TLB flush handling.

use super::block::Block;
#[cfg(test)]
use super::microop::MicroOp;
use std::collections::HashMap;

/// Block cache configuration.
pub const BLOCK_CACHE_SIZE: usize = 4096;

/// Block cache using PC as key.
pub struct BlockCache {
    /// PC → Block mapping.
    blocks: HashMap<u64, Box<Block>>,
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
        Self {
            blocks: HashMap::with_capacity(BLOCK_CACHE_SIZE),
            generation: 0,
            hits: 0,
            misses: 0,
            invalidations: 0,
        }
    }

    /// Look up a block by PC.
    /// Returns a reference to the block if found and valid.
    #[inline]
    pub fn get(&mut self, pc: u64) -> Option<&Block> {
        if let Some(block) = self.blocks.get(&pc) {
            if block.generation == self.generation {
                self.hits += 1;
                return Some(block);
            }
        }
        self.misses += 1;
        None
    }

    /// Insert a compiled block into the cache.
    pub fn insert(&mut self, block: Block) {
        // Evict if cache is full
        if self.blocks.len() >= BLOCK_CACHE_SIZE {
            self.evict_cold();
        }

        let pc = block.start_pc;
        self.blocks.insert(pc, Box::new(block));
    }

    /// Invalidate all blocks (called on SATP change, SFENCE.VMA).
    pub fn flush(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.invalidations += 1;
        // Don't clear the map; stale entries will be rejected by generation check
    }

    /// Invalidate blocks in a specific physical address range.
    /// Called when code is modified.
    pub fn invalidate_range(&mut self, start_pa: u64, end_pa: u64) {
        self.blocks.retain(|_, block| {
            let block_end = block.start_pa + block.byte_len as u64;
            !(block.start_pa < end_pa && block_end > start_pa)
        });
        self.invalidations += 1;
    }

    /// Evict least-used blocks when cache is full.
    fn evict_cold(&mut self) {
        // Simple strategy: remove blocks with lowest exec_count
        let threshold = self.blocks.len() / 4;
        let mut cold = Vec::with_capacity(threshold);

        for (&pc, block) in &self.blocks {
            if block.exec_count < 10 {
                cold.push(pc);
                if cold.len() >= threshold {
                    break;
                }
            }
        }

        for pc in cold {
            self.blocks.remove(&pc);
        }

        // If we didn't find enough cold blocks, remove oldest (by generation)
        if self.blocks.len() >= BLOCK_CACHE_SIZE {
            let oldest_gen = self.generation.wrapping_sub(1);
            self.blocks
                .retain(|_, block| block.generation >= oldest_gen);
        }
    }

    /// Get mutable block for updating exec_count.
    #[inline]
    pub fn get_mut(&mut self, pc: u64) -> Option<&mut Block> {
        self.blocks.get_mut(&pc).map(|b| &mut **b)
    }

    /// Look up a block and increment its exec_count in a single operation.
    /// This is more efficient than separate get() + get_mut() calls.
    /// Returns a reference to the block if found and valid.
    #[inline]
    pub fn get_and_touch(&mut self, pc: u64) -> Option<&Block> {
        if let Some(block) = self.blocks.get_mut(&pc) {
            if block.generation == self.generation {
                self.hits += 1;
                block.exec_count = block.exec_count.saturating_add(1);
                return Some(&**block);
            }
        }
        self.misses += 1;
        None
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.generation = 0;
        self.hits = 0;
        self.misses = 0;
        self.invalidations = 0;
    }

    /// Get cache statistics as a tuple: (hits, misses, size, hit_rate).
    pub fn stats(&self) -> (u64, u64, usize, f64) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
        (self.hits, self.misses, self.blocks.len(), hit_rate)
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
        let mut block = make_test_block(0x8000_0000, 0);
        cache.generation = 1; // Advance generation
        block.generation = 0; // Block has old generation
        cache.blocks.insert(0x8000_0000, Box::new(block));

        // Should not find it due to generation mismatch
        assert!(cache.get(0x8000_0000).is_none());
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
