// run with: cargo run --release --benchmarks bench

use std::path;
use std::time::{Duration, Instant};

use dir_search_rs::{ParseConfig, SearchContents, search_with_config};

const DIR: &str = "data/file-contents";
const NEEDLE: &str = "ZZNEEDLEZZ";
const ITERS: usize = 10;

fn serial_search(dir: &str, needle: &str) -> Vec<path::PathBuf> {
    let mut res = vec![];
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            println!("could not read {dir}: {err}");
            return res;
        }
    };
    for entry in entries.flatten() {
        if let Ok(contents) = std::fs::read_to_string(entry.path()) {
            if contents.contains(needle) {
                res.push(entry.path());
            }
        }
    }
    res
}

fn bench<F: Fn() -> usize>(f: F) -> (Duration, Duration, Duration, usize) {
    let hits = f(); // warmup

    let mut times = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let start = Instant::now();
        let got = f();
        times.push(start.elapsed());
        assert_eq!(got, hits, "result count changed between runs");
    }
    times.sort();

    let min = times[0];
    let median = times[ITERS / 2];
    let mean = times.iter().sum::<Duration>() / ITERS as u32;
    (min, median, mean, hits)
}

fn dir_stats(dir: &str) -> (usize, u64) {
    let mut count = 0;
    let mut bytes = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    count += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (count, bytes)
}

fn mb_per_s(bytes: u64, d: Duration) -> f64 {
    (bytes as f64 / 1_000_000.0) / d.as_secs_f64()
}

fn main() {
    let (count, bytes) = dir_stats(DIR);
    if count == 0 {
        println!("no files found in {DIR}/");
        std::process::exit(1);
    }

    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    println!("dir-search-rs contents-search benchmark");
    println!("  directory : {DIR}/");
    println!("  files     : {count}");
    println!("  total size: {:.1} MB", bytes as f64 / 1_000_000.0);
    println!("  needle    : {NEEDLE:?}");
    println!("  iterations: {ITERS} (after 1 warmup)");
    println!("  cores     : {cores} (library uses available_parallelism)");
    println!();

    let config = ParseConfig {
        search_dirs: vec![DIR.to_string()],
        search_strs: vec!["{search}".to_string()],
        search_contents: SearchContents::FileContents(None, false),
        parallel_preference: None,
    };
    let (p_min, p_med, p_mean, p_hits) =
        bench(|| search_with_config(&config, NEEDLE, None).unwrap().len());

    let (s_min, s_med, s_mean, s_hits) = bench(|| serial_search(DIR, NEEDLE).len());

    assert_eq!(
        p_hits, s_hits,
        "parallel and serial found different result counts"
    );

    let fmt = |d: Duration| format!("{:>8.2} ms", d.as_secs_f64() * 1000.0);
    println!(
        "{:<22} {:>11} {:>11} {:>11} {:>12}",
        "impl", "min", "median", "mean", "throughput"
    );
    println!(
        "{:<22} {} {} {} {:>9.0} MB/s",
        "serial (1 thread)",
        fmt(s_min),
        fmt(s_med),
        fmt(s_mean),
        mb_per_s(bytes, s_min),
    );
    println!(
        "{:<22} {} {} {} {:>9.0} MB/s",
        format!("parallel ({cores} thr)"),
        fmt(p_min),
        fmt(p_med),
        fmt(p_mean),
        mb_per_s(bytes, p_min),
    );
    println!();
    println!("hits: {p_hits}");
    println!(
        "speedup (median): {:.2}x",
        s_med.as_secs_f64() / p_med.as_secs_f64()
    );
}
