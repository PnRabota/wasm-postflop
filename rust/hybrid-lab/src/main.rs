use hybrid_lab::{run_bench_suite, BenchConfig};

fn main() {
    let config = parse_config();
    println!("hybrid-lab benchmark");
    println!(
        "config depth={} branching={} hands={} actions={} iters={} alignment={}",
        config.depth,
        config.branching,
        config.hand_count,
        config.action_count,
        config.iterations,
        config.alignment
    );

    let results = run_bench_suite(config);
    println!();
    println!(
        "{:<20} {:>12} {:>14} {:>14} {:>14} {:>10}",
        "backend", "time(ms)", "slot updates/s", "upload(MB)", "download(MB)", "status"
    );
    println!("{}", "-".repeat(96));
    for row in results {
        let upload_mb = row.transfer.upload_bytes as f64 / (1024.0 * 1024.0);
        let download_mb = row.transfer.download_bytes as f64 / (1024.0 * 1024.0);
        let status = if row.note.is_some() { "skipped" } else { "ok" };
        println!(
            "{:<20} {:>12.2} {:>14.0} {:>14.2} {:>14.2} {:>10}",
            row.name,
            row.elapsed.as_secs_f64() * 1000.0,
            row.slot_updates_per_sec(),
            upload_mb,
            download_mb,
            status
        );
        if let Some(note) = row.note {
            println!("  note: {note}");
        }
    }
}

fn parse_config() -> BenchConfig {
    let mut config = BenchConfig::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        let Some(value) = args.next() else {
            break;
        };
        match arg.as_str() {
            "--depth" => config.depth = parse_usize(&value, config.depth),
            "--branching" => config.branching = parse_usize(&value, config.branching),
            "--hands" => config.hand_count = parse_usize(&value, config.hand_count),
            "--actions" => config.action_count = parse_usize(&value, config.action_count),
            "--iters" => config.iterations = parse_u32(&value, config.iterations),
            "--align" => config.alignment = parse_usize(&value, config.alignment),
            _ => {}
        }
    }

    config
}

fn parse_usize(text: &str, fallback: usize) -> usize {
    text.parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn parse_u32(text: &str, fallback: u32) -> u32 {
    text.parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}
