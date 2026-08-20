use criterion::{black_box, criterion_group, criterion_main, Criterion};

use veridag_crypto::hash;

fn hash_domains(c: &mut Criterion) {
    let payloads: Vec<Vec<u8>> = (0..64)
        .map(|i| {
            // Deterministic payload per index so the benchmark is reproducible.
            let mut buf = vec![0u8; 256];
            buf[0] = i as u8;
            buf
        })
        .collect();

    let domains = [
        "VERIDAG_TX_V1",
        "VERIDAG_VERTEX_V1",
        "VERIDAG_BATCH_V1",
        "VERIDAG_CHECKPOINT_V1",
        "VERIDAG_DA_BLOB_V1",
    ];

    c.bench_function("hash_small_domain", |b| {
        b.iter(|| {
            for dom in &domains {
                for p in &payloads {
                    black_box(hash(dom, black_box(p.as_slice())));
                }
            }
        })
    });

    // Larger payload to stress the hasher.
    let big: Vec<u8> = (0..4096).map(|i| i as u8).collect();
    c.bench_function("hash_big_payload", |b| {
        b.iter(|| black_box(hash("VERIDAG_DA_BLOB_V1", black_box(&big))))
    });
}

criterion_group!(benches, hash_domains);
criterion_main!(benches);
