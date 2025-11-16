use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use qoxide_lib::QoxideQueue;
use std::hint::black_box;

const LARGE_PAYLOAD_SIZE: usize = 1000000; // 1MB

fn bench_queue_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_insert");
    group.throughput(Throughput::Elements(1));
    let mut queue = QoxideQueue::new();
    let payload = b"0".to_vec();
    group.bench_function("queue_insert", |b| {
        b.iter(|| queue.insert(black_box(payload.clone())))
    });

    let payload = vec![1; LARGE_PAYLOAD_SIZE];
    group.bench_function("queue_insert_large_payload", |b| {
        b.iter(|| queue.insert(black_box(payload.clone())))
    });
}

fn bench_queue_reserve(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_reserve");
    let payload = b"0".to_vec();
    group.throughput(Throughput::Elements(1));

    for &queue_size in &[10_000, 100_000] {
        group.bench_function(format!("reserve_queue({queue_size})"), |b| {
            b.iter_batched(
                || {
                    let mut queue = QoxideQueue::new();
                    for _ in 0..queue_size {
                        queue.insert(payload.clone());
                    }
                    queue
                },
                |mut queue| {
                    black_box(queue.reserve());
                },
                BatchSize::SmallInput,
            );
        });
    }

    let payload = vec![1; LARGE_PAYLOAD_SIZE];
    for &queue_size in &[10_000, 100_000] {
        group.bench_function(format!("reserve_queue({queue_size}_large_payload)"), |b| {
            b.iter_batched(
                || {
                    let mut queue = QoxideQueue::new();
                    for _ in 0..queue_size {
                        queue.insert(payload.clone());
                    }
                    queue
                },
                |mut queue| {
                    black_box(queue.reserve());
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(benches, bench_queue_insert, bench_queue_reserve);
criterion_main!(benches);
