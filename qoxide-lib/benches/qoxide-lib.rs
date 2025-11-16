use criterion::{Criterion, criterion_group, criterion_main};
use qoxide_lib::QoxideQueue;

fn bench_queue_insert(c: &mut Criterion) {
    let mut queue = QoxideQueue::new();
    let payload = b"test".to_vec();
    c.bench_function("queue_insert", |b| b.iter(|| queue.insert(payload.clone())));
}

criterion_group!(benches, bench_queue_insert);
criterion_main!(benches);
