#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};

fn start(_c: &mut Criterion) {}

criterion_group!(benches, start);
criterion_main!(benches);
