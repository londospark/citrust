use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use citrust::crypto::{rol128, derive_normal_key, aes_ctr_decrypt};

fn bench_rol128(c: &mut Criterion) {
    let mut group = c.benchmark_group("rol128");
    
    let value = 0x12345678_9ABCDEF0_FEDCBA98_76543210u128;
    
    group.bench_function("rol128_small_shift", |b| {
        b.iter(|| rol128(black_box(value), black_box(2)))
    });
    
    group.bench_function("rol128_large_shift", |b| {
        b.iter(|| rol128(black_box(value), black_box(87)))
    });
    
    group.finish();
}

fn bench_derive_normal_key(c: &mut Criterion) {
    let key_x = 0xB98E95CECA3E4D171F76A94DE934C053u128;
    let key_y = 0x12345678_9ABCDEF0_FEDCBA98_76543210u128;
    let constant = 0x1FF9E9AAC5FE0408024591DC5D52768Au128;
    
    c.bench_function("derive_normal_key", |b| {
        b.iter(|| {
            derive_normal_key(
                black_box(key_x),
                black_box(key_y),
                black_box(constant)
            )
        })
    });
}

fn bench_aes_ctr_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes_ctr_decrypt");
    
    let key: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
    ];
    let iv = 0xf0f1f2f3f4f5f6f7f8f9fafbfcfdfeFFu128;
    
    // 1 MB buffer
    let mb = 1024 * 1024;
    let mut data_1mb = vec![0u8; mb];
    group.throughput(Throughput::Bytes(mb as u64));
    group.bench_with_input(BenchmarkId::new("1MB", mb), &mut data_1mb, |b, data| {
        b.iter(|| {
            let mut buf = data.clone();
            aes_ctr_decrypt(black_box(&key), black_box(iv), black_box(&mut buf));
        })
    });
    
    // 16 MB buffer
    let mb16 = 16 * 1024 * 1024;
    let mut data_16mb = vec![0u8; mb16];
    group.throughput(Throughput::Bytes(mb16 as u64));
    group.bench_with_input(BenchmarkId::new("16MB", mb16), &mut data_16mb, |b, data| {
        b.iter(|| {
            let mut buf = data.clone();
            aes_ctr_decrypt(black_box(&key), black_box(iv), black_box(&mut buf));
        })
    });
    
    group.finish();
}

fn bench_key_derivation_pipeline(c: &mut Criterion) {
    c.bench_function("full_key_derivation", |b| {
        let key_x = 0xB98E95CECA3E4D171F76A94DE934C053u128;
        let key_y = 0x12345678_9ABCDEF0_FEDCBA98_76543210u128;
        let constant = 0x1FF9E9AAC5FE0408024591DC5D52768Au128;
        
        b.iter(|| {
            let normal_key = derive_normal_key(
                black_box(key_x),
                black_box(key_y),
                black_box(constant)
            );
            black_box(normal_key.to_be_bytes())
        })
    });
}

criterion_group!(
    benches,
    bench_rol128,
    bench_derive_normal_key,
    bench_aes_ctr_decrypt,
    bench_key_derivation_pipeline
);
criterion_main!(benches);
