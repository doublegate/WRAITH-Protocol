use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use wraith_core::frame::compat::{v1_header_to_v2, v2_header_to_v1};
use wraith_core::{
    ConnectionId, ConnectionIdV2, FRAME_HEADER_SIZE, FlagsV2, FormatNegotiation, Frame,
    FrameBuilder, FrameFlags, FrameHeaderV2, FrameType, FrameTypeV2, PolymorphicFormat,
    build_into_from_parts, detect_format,
};

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

fn bench_frame_build_into(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (128, "128_bytes"),
        (256, "256_bytes"),
        (512, "512_bytes"),
        (1024, "1024_bytes"),
        (1456, "1456_bytes"),
    ];

    let mut group = c.benchmark_group("frame_build_into");

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let payload = vec![0x42; payload_len];
        let builder = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .payload(&payload);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(name, |b| {
            let mut buf = vec![0u8; size];
            b.iter(|| builder.build_into(black_box(&mut buf)))
        });
    }

    group.finish();
}

fn bench_frame_full_pipeline(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (256, "256_bytes"),
        (1024, "1024_bytes"),
        (1456, "1456_bytes"),
    ];

    let mut group = c.benchmark_group("frame_full_pipeline");

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let payload = vec![0x42; payload_len];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(name, |b| {
            b.iter(|| {
                let frame = FrameBuilder::new()
                    .frame_type(black_box(FrameType::Data))
                    .stream_id(black_box(42))
                    .sequence(black_box(1000))
                    .payload(black_box(&payload))
                    .build(black_box(size))
                    .unwrap();

                let parsed = Frame::parse(black_box(&frame)).unwrap();
                black_box(parsed.payload().len())
            })
        });
    }

    group.finish();
}

fn bench_frame_build_into_from_parts(c: &mut Criterion) {
    let sizes: Vec<(usize, &str)> = vec![
        (64, "64_bytes"),
        (256, "256_bytes"),
        (512, "512_bytes"),
        (1024, "1024_bytes"),
        (1456, "1456_bytes"),
    ];

    let mut group = c.benchmark_group("frame_build_into_from_parts");

    for (size, name) in sizes {
        let payload_len = size.saturating_sub(FRAME_HEADER_SIZE);
        let payload = vec![0x42; payload_len];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(name, |b| {
            let mut buf = vec![0u8; size];
            b.iter(|| {
                build_into_from_parts(
                    black_box(FrameType::Data),
                    black_box(42),
                    black_box(1000),
                    black_box(0),
                    black_box(&payload),
                    black_box(&mut buf),
                )
            })
        });
    }

    group.finish();
}

// ============================================================================
// v2 Wire Format Benchmarks
// ============================================================================

fn bench_connection_id_v2(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_id_v2");

    group.bench_function("generate", |b| {
        b.iter(|| black_box(ConnectionIdV2::generate()))
    });

    let v1 = ConnectionId::from_bytes([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
    group.bench_function("from_v1", |b| {
        b.iter(|| black_box(ConnectionIdV2::from_v1(black_box(v1))))
    });

    let cid = ConnectionIdV2::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    group.bench_function("rotate", |b| {
        b.iter(|| black_box(black_box(cid).rotate(black_box(0x1111111111111111u64))))
    });

    group.bench_function("write_read_roundtrip", |b| {
        let cid = ConnectionIdV2::generate();
        let mut buf = [0u8; 16];
        b.iter(|| {
            black_box(cid).write_to(black_box(&mut buf));
            black_box(ConnectionIdV2::read_from(black_box(&buf)))
        })
    });

    group.bench_function("is_valid", |b| {
        b.iter(|| black_box(black_box(cid).is_valid()))
    });

    group.bench_function("is_migrated_v1", |b| {
        let migrated = ConnectionIdV2::from_v1(v1);
        b.iter(|| black_box(black_box(migrated).is_migrated_v1()))
    });

    group.finish();
}

fn test_header_v2() -> FrameHeaderV2 {
    FrameHeaderV2 {
        version: 0x20,
        frame_type: FrameTypeV2::StreamData,
        flags: FlagsV2::empty().with(FlagsV2::SYN).with(FlagsV2::ECN),
        sequence: 0x0123_4567_89AB_CDEF,
        length: 1400,
        stream_id: 42,
        reserved: 0,
    }
}

fn bench_header_v2(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_v2");
    group.throughput(Throughput::Bytes(24));

    let header = test_header_v2();

    group.bench_function("encode", |b| {
        b.iter(|| black_box(black_box(header).encode()))
    });

    let encoded = header.encode();
    group.bench_function("decode", |b| {
        b.iter(|| FrameHeaderV2::decode(black_box(&encoded)))
    });

    #[cfg(feature = "simd")]
    group.bench_function("decode_simd", |b| {
        b.iter(|| FrameHeaderV2::decode_simd(black_box(&encoded)))
    });

    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let enc = black_box(header).encode();
            black_box(FrameHeaderV2::decode(black_box(&enc)))
        })
    });

    group.bench_function("encode_into", |b| {
        let mut buf = [0u8; 24];
        b.iter(|| header.encode_into(black_box(&mut buf)))
    });

    group.bench_function("new", |b| {
        b.iter(|| black_box(FrameHeaderV2::new(black_box(FrameTypeV2::Data))))
    });

    group.finish();
}

fn bench_polymorphic(c: &mut Criterion) {
    let mut group = c.benchmark_group("polymorphic");
    let secret = [0x42u8; 32];

    group.bench_function("derive", |b| {
        b.iter(|| black_box(PolymorphicFormat::derive(black_box(&secret))))
    });

    let fmt = PolymorphicFormat::derive(&secret);
    let header = test_header_v2();

    group.throughput(Throughput::Bytes(24));

    group.bench_function("encode_header", |b| {
        b.iter(|| black_box(fmt.encode_header(black_box(&header))))
    });

    let encoded = fmt.encode_header(&header);
    group.bench_function("decode_header", |b| {
        b.iter(|| fmt.decode_header(black_box(&encoded)))
    });

    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let enc = fmt.encode_header(black_box(&header));
            black_box(fmt.decode_header(black_box(&enc)))
        })
    });

    group.finish();
}

fn bench_compat(c: &mut Criterion) {
    let mut group = c.benchmark_group("compat");

    // detect_format with v2 data
    let v2_data = test_header_v2().encode();
    group.bench_function("detect_format_v2", |b| {
        b.iter(|| black_box(detect_format(black_box(&v2_data))))
    });

    // detect_format with v1 data
    let mut v1_data = vec![0u8; 28];
    v1_data[8] = 0x01; // Data frame type for v1
    group.bench_function("detect_format_v1", |b| {
        b.iter(|| black_box(detect_format(black_box(&v1_data))))
    });

    // v1_header_to_v2
    let v1_header = wraith_core::frame::FrameHeader {
        frame_type: FrameType::Data,
        flags: FrameFlags::new().with_syn(),
        stream_id: 42,
        sequence: 1000,
        offset: 8192,
        payload_len: 1400,
    };
    group.bench_function("v1_header_to_v2", |b| {
        b.iter(|| black_box(v1_header_to_v2(black_box(&v1_header))))
    });

    // v2_header_to_v1
    let v2_header = test_header_v2();
    group.bench_function("v2_header_to_v1", |b| {
        b.iter(|| black_box(v2_header_to_v1(black_box(&v2_header))))
    });

    // FormatNegotiation
    let local = FormatNegotiation::default();
    let remote = FormatNegotiation::default();
    group.bench_function("negotiate", |b| {
        b.iter(|| black_box(black_box(local).negotiate(black_box(&remote))))
    });

    group.finish();
}

fn bench_frame_type_v2(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_type_v2");

    group.bench_function("try_from_valid", |b| {
        b.iter(|| FrameTypeV2::try_from(black_box(0x31u8))) // StreamData
    });

    group.bench_function("try_from_invalid", |b| {
        b.iter(|| FrameTypeV2::try_from(black_box(0xFFu8)))
    });

    group.bench_function("is_valid_byte", |b| {
        b.iter(|| FrameTypeV2::is_valid_byte(black_box(0x31u8)))
    });

    group.bench_function("category", |b| {
        let ft = FrameTypeV2::StreamData;
        b.iter(|| black_box(black_box(ft).category()))
    });

    group.bench_function("is_data", |b| {
        let ft = FrameTypeV2::Data;
        b.iter(|| black_box(black_box(ft).is_data()))
    });

    // FlagsV2 operations
    group.bench_function("flags_with", |b| {
        b.iter(|| {
            black_box(
                FlagsV2::empty()
                    .with(FlagsV2::SYN)
                    .with(FlagsV2::ECN)
                    .with(FlagsV2::CMP),
            )
        })
    });

    group.bench_function("flags_contains", |b| {
        let flags = FlagsV2::from_bits(0x00FF);
        b.iter(|| black_box(black_box(flags).contains(black_box(FlagsV2::ECN))))
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
    bench_parse_throughput,
    bench_frame_build_into,
    bench_frame_build_into_from_parts,
    bench_frame_full_pipeline
);

criterion_group!(
    v2_benches,
    bench_connection_id_v2,
    bench_header_v2,
    bench_polymorphic,
    bench_compat,
    bench_frame_type_v2
);

criterion_main!(benches, v2_benches);
