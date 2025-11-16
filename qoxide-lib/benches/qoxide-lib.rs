use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use qoxide_lib::QoxideQueue;
use std::hint::black_box;

const LARGE_PAYLOAD_SIZE: usize = 1000000; // 1MB
const QUEUE_SIZES: [usize; 3] = [1000, 10_000, 100_000];

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

    for &queue_size in &QUEUE_SIZES {
        group.bench_with_input(
            BenchmarkId::from_parameter(queue_size),
            &queue_size,
            |b, &queue_size| {
                b.iter_batched(
                    || {
                        let mut queue = QoxideQueue::new();
                        for _ in 0..queue_size {
                            queue.insert(black_box(payload.clone()));
                        }
                        queue
                    },
                    |mut queue| {
                        black_box(queue.reserve());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_queue_interactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_interactions");
    group.throughput(Throughput::Elements(1));
    let mut queue = QoxideQueue::new();
    let payload = b"0".to_vec();
    group.bench_function("queue_interactions", |b| {
        b.iter(|| {
            queue.insert(black_box(payload.clone()));
            let (id, _) = queue.reserve().expect("Message should be found");
            queue.fail(id);
            let (id, _) = queue.reserve().expect("Message should be found");
            queue.complete(id);
        })
    });
}

criterion_group!(
    benches,
    bench_queue_insert,
    bench_queue_reserve,
    bench_queue_interactions
);
criterion_main!(benches);
