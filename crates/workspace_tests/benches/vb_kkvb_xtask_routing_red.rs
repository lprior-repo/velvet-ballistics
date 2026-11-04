use criterion::{Criterion, criterion_group, criterion_main};
use std::ffi::OsString;
use std::hint::black_box;
use std::path::PathBuf;
use xtask::{
    CommandFamily, OutputFormat, XtaskCommand, XtaskEnvironment, parse_xtask_command, route_command,
};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;measurement=criterion-timing;external-counters=separate-evidence";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

fn benchmark_parse_required_command(c: &mut Criterion) {
    let fixture_bytes: usize = [
        "xtask",
        "ai-context",
        "--bead",
        "vb-kkvb",
        "--format",
        "jsonl",
    ]
    .iter()
    .map(|arg| arg.len())
    .sum();
    let id = metadata(
        "vb_kkvb_parse_ai_context",
        fixture_bytes,
        "fixture=argv_6_args;surface=parse_ai_context",
    );
    c.bench_function(&id, |bench| {
        bench.iter(|| {
            black_box(parse_xtask_command(black_box([
                OsString::from("xtask"),
                OsString::from("ai-context"),
                OsString::from("--bead"),
                OsString::from("vb-kkvb"),
                OsString::from("--format"),
                OsString::from("jsonl"),
            ])))
        })
    });
}

fn benchmark_route_deferred_command(c: &mut Criterion) {
    let env = XtaskEnvironment {
        workspace_root: PathBuf::from("/workspace"),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families: Vec::new(),
    };
    let command = XtaskCommand::Required(CommandFamily::AiContext);

    let id = metadata(
        "vb_kkvb_route_ai_context_deferred",
        1,
        "fixture=route_1_command;surface=route_ai_context_deferred",
    );
    c.bench_function(&id, |bench| {
        bench.iter(|| black_box(route_command(black_box(command.clone()), black_box(&env))))
    });
}

criterion_group!(
    benches,
    benchmark_parse_required_command,
    benchmark_route_deferred_command
);
criterion_main!(benches);
