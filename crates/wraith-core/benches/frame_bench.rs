use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use wraith_core::{FRAME_HEADER_SIZE, Frame, FrameBuilder, FrameType};

fn bench_frame_parse(c: &mut Criterion) {
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(42)
        .sequence(1000)
        .offset(0)
        .payload(&vec![0xAA; 1200])
        .build(1456)
        .unwrap();

    let mut group = c.benchmark_group("frame_parse");
    group.throughput(Throughput::Bytes(frame_data.len() as u64));

    group.bench_function("parse_1456_bytes", |b| {
        b.iter(|| Frame::parse(black_box(&frame_data)))
    });

    group.finish();
}

fn bench_frame_parse_sizes(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (128, "128_bytes"),
        (256, "256_bytes"),
        (512, "512_bytes"),
        (1024, "1024_bytes"),
        (1456, "1456_bytes"),
    ];

    let mut group = c.benchmark_group("frame_parse_by_size");

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let frame_data = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(&vec![0x42; payload_len])
            .build(size)
            .unwrap();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(name, |b| b.iter(|| Frame::parse(black_box(&frame_data))));
    }

    group.finish();
}

fn bench_frame_build(c: &mut Criterion) {
    let payload = vec![0xBB; 1200];

    let mut group = c.benchmark_group("frame_build");
    group.throughput(Throughput::Bytes(1456));

    group.bench_function("build_1456_bytes", |b| {
        b.iter(|| {
            FrameBuilder::new()
                .frame_type(black_box(FrameType::Data))
                .stream_id(black_box(42))
                .sequence(black_box(1000))
                .payload(black_box(&payload))
                .build(black_box(1456))
        })
    });

    group.finish();
}

fn bench_frame_build_sizes(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (128, "128_bytes"),
        (256, "256_bytes"),
        (512, "512_bytes"),
        (1024, "1024_bytes"),
        (1456, "1456_bytes"),
    ];

    let mut group = c.benchmark_group("frame_build_by_size");

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let payload = vec![0x42; payload_len];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(name, |b| {
            b.iter(|| {
                FrameBuilder::new()
                    .frame_type(black_box(FrameType::Data))
                    .payload(black_box(&payload))
                    .build(black_box(size))
            })
        });
    }

    group.finish();
}

fn bench_frame_roundtrip(c: &mut Criterion) {
    let payload = vec![0xCC; 1200];

    let mut group = c.benchmark_group("frame_roundtrip");
    group.throughput(Throughput::Bytes(1456));

    group.bench_function("build_and_parse", |b| {
        b.iter(|| {
            let frame = FrameBuilder::new()
                .frame_type(black_box(FrameType::Data))
                .stream_id(black_box(42))
                .sequence(black_box(1000))
                .payload(black_box(&payload))
                .build(black_box(1456))
                .unwrap();

            let parsed = Frame::parse(black_box(&frame)).unwrap();
            // Consume the parsed frame to prevent optimization
            black_box(parsed.frame_type())
        })
    });

    group.finish();
}

fn bench_frame_types(c: &mut Criterion) {
    let frame_types = vec![
        (FrameType::Data, "data"),
        (FrameType::Ack, "ack"),
        (FrameType::Ping, "ping"),
        (FrameType::StreamOpen, "stream_open"),
    ];

    let mut group = c.benchmark_group("frame_types");

    for (ft, name) in frame_types {
        let frame_data = FrameBuilder::new()
            .frame_type(ft)
            .payload(&[0u8; 64])
            .build(128)
            .unwrap();

        group.bench_function(name, |b| b.iter(|| Frame::parse(black_box(&frame_data))));
    }

    group.finish();
}

fn bench_scalar_vs_simd(c: &mut Criterion) {
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(42)
        .sequence(1000)
        .offset(0)
        .payload(&vec![0xAA; 1200])
        .build(1456)
        .unwrap();

    let mut group = c.benchmark_group("scalar_vs_simd");
    group.throughput(Throughput::Bytes(frame_data.len() as u64));

    group.bench_function("scalar", |b| {
        b.iter(|| Frame::parse_scalar(black_box(&frame_data)))
    });

    #[cfg(feature = "simd")]
    group.bench_function("simd", |b| {
        b.iter(|| Frame::parse_simd(black_box(&frame_data)))
    });

    group.bench_function("default", |b| {
        b.iter(|| Frame::parse(black_box(&frame_data)))
    });

    group.finish();
}

fn bench_parse_implementations_by_size(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (128, "128_bytes"),
        (512, "512_bytes"),
        (1456, "1456_bytes"),
    ];

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let frame_data = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(&vec![0x42; payload_len])
            .build(size)
            .unwrap();

        let mut group = c.benchmark_group(format!("parse_impl_{}", name));
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_function("scalar", |b| {
            b.iter(|| Frame::parse_scalar(black_box(&frame_data)))
        });

        #[cfg(feature = "simd")]
        group.bench_function("simd", |b| {
            b.iter(|| Frame::parse_simd(black_box(&frame_data)))
        });

        group.finish();
    }
}

fn bench_parse_throughput(c: &mut Criterion) {
    // Benchmark parsing throughput (frames per second)
    let frame_data = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .stream_id(1)
        .sequence(1)
        .payload(&vec![0xBB; 1200])
        .build(1456)
        .unwrap();

    let mut group = c.benchmark_group("parse_throughput");
    group.throughput(Throughput::Elements(1)); // Measure frames/sec

    group.bench_function("scalar_fps", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let _ = Frame::parse_scalar(black_box(&frame_data));
            }
        })
    });

    #[cfg(feature = "simd")]
    group.bench_function("simd_fps", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let _ = Frame::parse_simd(black_box(&frame_data));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_frame_parse,
    bench_frame_parse_sizes,
    bench_frame_build,
    bench_frame_build_sizes,
    bench_frame_roundtrip,
    bench_frame_types,
    bench_scalar_vs_simd,
    bench_parse_implementations_by_size,
    bench_parse_throughput
);
criterion_main!(benches);
