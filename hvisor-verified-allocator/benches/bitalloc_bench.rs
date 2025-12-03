use criterion::{criterion_group, criterion_main, Criterion};
use hvisor_verified_allocator::original as v1;
use hvisor_verified_allocator::verified_impl as v2;
use hvisor_verified_allocator::optimized as v3;

/// ------------------------
/// Program level: Four test suites
/// ------------------------
pub fn bench_program_suites(c: &mut Criterion) {
    // bitalloc16
    {
        let mut group = c.benchmark_group("suite_bitalloc16");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc16()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc16()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc16()));
        group.finish();
    }

    // bitalloc4k
    {
        let mut group = c.benchmark_group("suite_bitalloc4k");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc4k()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc4k()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc4k()));
        group.finish();
    }

    // bitalloc_contiguous
    {
        let mut group = c.benchmark_group("suite_bitalloc_contiguous");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc_contiguous()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc_contiguous()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc_contiguous()));
        group.finish();
    }

    // bitalloc1m
    {
        let mut group = c.benchmark_group("suite_bitalloc1m");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m()));
        group.finish();
    }
}

/// --------------------------------------
/// Function level: The key operation function of bitalloc1m
/// --------------------------------------
pub fn bench_bitalloc1m_functions(c: &mut Criterion) {
    // alloc
    {
        let mut group = c.benchmark_group("func_bitalloc1m_alloc");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m_alloc()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m_alloc()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m_alloc()));
        group.finish();
    }

    // alloc_contiguous
    {
        let mut group = c.benchmark_group("func_bitalloc1m_alloc_contiguous");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m_alloc_contiguous()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m_alloc_contiguous()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m_alloc_contiguous()));
        group.finish();
    }

    // dealloc
    {
        let mut group = c.benchmark_group("func_bitalloc1m_dealloc");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m_dealloc()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m_dealloc()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m_dealloc()));
        group.finish();
    }

    // insert
    {
        let mut group = c.benchmark_group("func_bitalloc1m_insert");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m_insert()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m_insert()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m_insert()));
        group.finish();
    }

    // remove
    {
        let mut group = c.benchmark_group("func_bitalloc1m_remove");
        group.bench_function("v1_original", |b| b.iter(|| v1::bitalloc1m_remove()));
        group.bench_function("v2_verified", |b| b.iter(|| v2::bitalloc1m_remove()));
        group.bench_function("v3_optimized", |b| b.iter(|| v3::bitalloc1m_remove()));
        group.finish();
    }
}

criterion_group!(benches, bench_program_suites, bench_bitalloc1m_functions);
criterion_main!(benches);