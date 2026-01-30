//! Benchmarks for WRAITH obfuscation layer.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use wraith_obfuscation::*;

fn bench_padding(c: &mut Criterion) {
    let mut group = c.benchmark_group("padding");

    for size in [128, 512, 1024, 4096] {
        let data = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("size_classes_{}", size), &data, |b, data| {
            let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
            b.iter(|| {
                let mut buf = data.clone();
                let target = engine.padded_size(data.len());
                engine.pad(&mut buf, target);
                black_box(buf);
            });
        });

        group.bench_with_input(format!("statistical_{}", size), &data, |b, data| {
            let mut engine = PaddingEngine::new(PaddingMode::Statistical);
            b.iter(|| {
                let mut buf = data.clone();
                let target = engine.padded_size(data.len());
                engine.pad(&mut buf, target);
                black_box(buf);
            });
        });
    }

    group.finish();
}

fn bench_tls_wrap(c: &mut Criterion) {
    let mut group = c.benchmark_group("tls_mimicry");

    for size in [128, 512, 1024, 4096] {
        let payload = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("wrap_{}", size), &payload, |b, payload| {
            let mut wrapper = TlsRecordWrapper::new();
            b.iter(|| {
                let wrapped = wrapper.wrap(black_box(payload));
                black_box(wrapped);
            });
        });

        group.bench_with_input(format!("unwrap_{}", size), &payload, |b, payload| {
            let mut wrapper = TlsRecordWrapper::new();
            let record = wrapper.wrap(payload);
            b.iter(|| {
                let unwrapped = wrapper.unwrap(black_box(&record)).unwrap();
                black_box(unwrapped);
            });
        });
    }

    group.finish();
}

fn bench_websocket_wrap(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_mimicry");

    for size in [128, 512, 1024, 4096] {
        let payload = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("wrap_server_{}", size), &payload, |b, payload| {
            let wrapper = WebSocketFrameWrapper::new(false);
            b.iter(|| {
                let wrapped = wrapper.wrap(black_box(payload));
                black_box(wrapped);
            });
        });

        group.bench_with_input(format!("wrap_client_{}", size), &payload, |b, payload| {
            let wrapper = WebSocketFrameWrapper::new(true);
            b.iter(|| {
                let wrapped = wrapper.wrap(black_box(payload));
                black_box(wrapped);
            });
        });
    }

    group.finish();
}

fn bench_doh_tunnel(c: &mut Criterion) {
    let mut group = c.benchmark_group("doh_tunnel");

    for size in [128, 512, 1024, 4096] {
        let payload = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("create_query_{}", size), &payload, |b, payload| {
            let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
            b.iter(|| {
                let query = tunnel
                    .create_dns_query(black_box("test.com"), black_box(payload))
                    .unwrap();
                black_box(query);
            });
        });

        group.bench_with_input(
            format!("parse_response_{}", size),
            &payload,
            |b, payload| {
                let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
                let query = tunnel.create_dns_query("test.com", payload).unwrap();
                b.iter(|| {
                    let parsed = tunnel.parse_dns_response(black_box(&query)).unwrap();
                    black_box(parsed);
                });
            },
        );
    }

    group.finish();
}

fn bench_timing_obfuscator(c: &mut Criterion) {
    let mut group = c.benchmark_group("timing");

    use std::time::Duration;

    group.bench_function("none", |b| {
        let mut obfuscator = TimingObfuscator::new(TimingMode::None);
        b.iter(|| {
            let delay = obfuscator.next_delay();
            black_box(delay);
        });
    });

    group.bench_function("fixed", |b| {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Fixed(Duration::from_millis(10)));
        b.iter(|| {
            let delay = obfuscator.next_delay();
            black_box(delay);
        });
    });

    group.bench_function("uniform", |b| {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Uniform {
            min: Duration::from_millis(5),
            max: Duration::from_millis(15),
        });
        b.iter(|| {
            let delay = obfuscator.next_delay();
            black_box(delay);
        });
    });

    group.bench_function("normal", |b| {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Normal {
            mean: Duration::from_millis(10),
            stddev: Duration::from_millis(2),
        });
        b.iter(|| {
            let delay = obfuscator.next_delay();
            black_box(delay);
        });
    });

    group.bench_function("exponential", |b| {
        let mut obfuscator = TimingObfuscator::new(TimingMode::Exponential {
            mean: Duration::from_millis(10),
        });
        b.iter(|| {
            let delay = obfuscator.next_delay();
            black_box(delay);
        });
    });

    group.finish();
}

fn bench_adaptive_profile(c: &mut Criterion) {
    c.bench_function("profile_from_threat_level", |b| {
        b.iter(|| {
            let profile = ObfuscationProfile::from_threat_level(black_box(ThreatLevel::High));
            black_box(profile);
        });
    });

    c.bench_function("profile_estimated_overhead", |b| {
        let profile = ObfuscationProfile::from_threat_level(ThreatLevel::High);
        b.iter(|| {
            let overhead = profile.estimated_overhead();
            black_box(overhead);
        });
    });
}

criterion_group!(
    benches,
    bench_padding,
    bench_tls_wrap,
    bench_websocket_wrap,
    bench_doh_tunnel,
    bench_timing_obfuscator,
    bench_adaptive_profile
);
criterion_main!(benches);
