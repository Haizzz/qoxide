use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use qoxide_lib::QoxideQueue;
use std::hint::black_box;

const QUEUE_SIZES: [u64; 2] = [100000, 1000000]; // 100K, 1M
const LARGE_PAYLOAD_SIZE: usize = 1000000; // 1MB

fn setup_queue(queue_size: u64, payload: &Vec<u8>) -> QoxideQueue {
    let mut queue = black_box(QoxideQueue::new());
    for _ in 0..queue_size {
        queue.insert(black_box(payload.clone()));
    }
    queue
}

fn bench_queue_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_insert");
    group.throughput(Throughput::Elements(1000000));
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

criterion_group!(benches, bench_queue_insert,);
criterion_main!(benches);
