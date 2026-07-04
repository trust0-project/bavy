//! Micro-benchmarks for the emulator's hot paths.
//!
//! These isolate the cost of individual subsystems (DRAM access, MMU
//! translation, block-cache lookup, instruction dispatch) so regressions
//! that end-to-end MIPS numbers hide are caught early.
//!
//! Run with: cargo bench -p riscv-vm

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

use riscv_vm::Mode;
use riscv_vm::bench::{WORKLOAD_DRAM_SIZE, workload_binary};
use riscv_vm::bus::{DRAM_BASE, SystemBus};
use riscv_vm::cpu::Cpu;
use riscv_vm::engine::block::Block;
use riscv_vm::engine::cache::BlockCache;
use riscv_vm::engine::microop::MicroOp;
use riscv_vm::mmu::{self, AccessType, Tlb};

fn bench_dram(c: &mut Criterion) {
    let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
    let mut group = c.benchmark_group("dram");
    group.throughput(Throughput::Elements(1));

    group.bench_function("load_64", |b| {
        let mut offset = 0u64;
        b.iter(|| {
            offset = (offset + 64) & 0xF_FFF8;
            black_box(bus.dram.load_64(black_box(offset)).unwrap())
        })
    });

    group.bench_function("store_64", |b| {
        let mut offset = 0u64;
        b.iter(|| {
            offset = (offset + 64) & 0xF_FFF8;
            bus.dram.store_64(black_box(offset), black_box(0xDEAD_BEEF)).unwrap()
        })
    });

    group.bench_function("load_32", |b| {
        let mut offset = 0u64;
        b.iter(|| {
            offset = (offset + 64) & 0xF_FFFC;
            black_box(bus.dram.load_32(black_box(offset)).unwrap())
        })
    });

    group.finish();
}

/// Build a minimal Sv39 page table in DRAM mapping a 1 GiB gigapage that
/// covers DRAM_BASE, and return the satp value for it.
fn setup_sv39(bus: &SystemBus) -> u64 {
    let root_pa: u64 = DRAM_BASE + 0x4000;
    let vpn2 = (DRAM_BASE >> 30) & 0x1FF;
    // Gigapage PTE: ppn = pa >> 12 with low 18 bits zero; flags V|R|W|X|A|D.
    let pte: u64 = ((DRAM_BASE >> 12) << 10) | 0xCF;
    bus.dram
        .store_64(root_pa - DRAM_BASE + vpn2 * 8, pte)
        .unwrap();
    (8u64 << 60) | (root_pa >> 12)
}

fn bench_mmu(c: &mut Criterion) {
    let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
    let satp = setup_sv39(&bus);
    let mut group = c.benchmark_group("mmu");
    group.throughput(Throughput::Elements(1));

    group.bench_function("translate_bare", |b| {
        let mut tlb = Tlb::new();
        b.iter(|| {
            mmu::translate(
                &bus,
                &mut tlb,
                Mode::Supervisor,
                black_box(0),
                0,
                black_box(DRAM_BASE + 0x1000),
                AccessType::Load,
            )
            .unwrap()
        })
    });

    group.bench_function("translate_sv39_tlb_hit", |b| {
        let mut tlb = Tlb::new();
        // Prime the TLB.
        mmu::translate(
            &bus,
            &mut tlb,
            Mode::Supervisor,
            satp,
            0,
            DRAM_BASE + 0x1000,
            AccessType::Load,
        )
        .unwrap();
        b.iter(|| {
            mmu::translate(
                &bus,
                &mut tlb,
                Mode::Supervisor,
                black_box(satp),
                0,
                black_box(DRAM_BASE + 0x1000),
                AccessType::Load,
            )
            .unwrap()
        })
    });

    group.bench_function("translate_sv39_walk", |b| {
        let mut tlb = Tlb::new();
        b.iter(|| {
            tlb.flush();
            mmu::translate(
                &bus,
                &mut tlb,
                Mode::Supervisor,
                black_box(satp),
                0,
                black_box(DRAM_BASE + 0x1000),
                AccessType::Load,
            )
            .unwrap()
        })
    });

    group.finish();
}

fn bench_block_cache(c: &mut Criterion) {
    let mut cache = BlockCache::new();
    // Populate with a realistic number of blocks.
    for i in 0..1024u64 {
        let pc = DRAM_BASE + i * 64;
        let mut block = Block::new(pc, pc, cache.generation);
        block.push(MicroOp::Addi { rd: 1, rs1: 0, imm: 1 }, 4);
        cache.insert(block);
    }

    let mut group = c.benchmark_group("block_cache");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_and_touch_hit", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i = (i + 1) % 1024;
            black_box(cache.get_and_touch(black_box(DRAM_BASE + i * 64)).is_some())
        })
    });

    group.bench_function("get_and_touch_miss", |b| {
        b.iter(|| black_box(cache.get_and_touch(black_box(0x1000)).is_none()))
    });

    group.finish();
}

fn bench_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("step");

    for workload in ["nop", "prime", "memcpy"] {
        let binary = workload_binary(workload, DRAM_BASE).unwrap();
        let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
        bus.set_num_harts(1);
        bus.dram.load(&binary, 0).unwrap();
        let mut cpu = Cpu::new(DRAM_BASE, 0);
        // Warm the block cache.
        for _ in 0..10_000 {
            let _ = cpu.step(&bus);
        }
        let start_instret = cpu.instret;
        let mut steps = 0u64;
        group.throughput(Throughput::Elements(1000));
        group.bench_function(format!("{workload}_1k_steps"), |b| {
            b.iter(|| {
                for _ in 0..1000 {
                    let _ = cpu.step(&bus);
                }
                steps += 1000;
            })
        });
        // Sanity: instret advanced (block engine active).
        assert!(cpu.instret > start_instret);
    }

    group.finish();
}

criterion_group!(benches, bench_dram, bench_mmu, bench_block_cache, bench_step);
criterion_main!(benches);
