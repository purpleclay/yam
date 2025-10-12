use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{fs, hint::black_box, path::Path};

fn parse_helm_values(c: &mut Criterion) {
    let fixtures = vec![
        // (
        //     "benches/fixtures/helm/external-dns.yaml",
        //     "external_dns_values",
        // ),
        ("benches/fixtures/helm/minio.yaml", "minio_values"),
        // ("benches/fixtures/helm/redis.yaml", "redis_values"),
    ];

    let mut group = c.benchmark_group("parse_yaml_files");

    for (file_path, name) in &fixtures {
        if !Path::new(file_path).exists() {
            eprintln!("fixture file {} not found, skipping", file_path);
            continue;
        }

        let yaml_content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("failed to read {}", file_path));

        let lines = yaml_content.lines().count();
        let size_kb = yaml_content.len() / 1024;

        group.bench_with_input(
            BenchmarkId::new(*name, format!("{}_kb_{}_lines", size_kb, lines)),
            &yaml_content,
            |b, content| {
                b.iter(|| {
                    let doc = yam::parser::parse(black_box(content))
                        .expect("parsing should not fail")
                        .expect("document should not be empty");
                    black_box(doc);
                })
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(10));
    targets = parse_helm_values
}
criterion_main!(benches);
