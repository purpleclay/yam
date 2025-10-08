use criterion::{Criterion, criterion_group, criterion_main};
use std::{fs, hint::black_box};

fn parse_helm_values(c: &mut Criterion) {
    let yaml_content =
        fs::read_to_string("benches/fixtures/values.yaml").expect("failed to read YAML file");

    let size_kb = yaml_content.len() / 1024;
    let lines = yaml_content.lines().count();

    c.bench_function(
        &format!("parse_helm_values_{}kb_{}_lines", size_kb, lines),
        |b| {
            b.iter(|| {
                let _ = yam::parser::parse(black_box(&yaml_content));
            })
        },
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(std::time::Duration::from_secs(7));
    targets = parse_helm_values
}
criterion_main!(benches);
