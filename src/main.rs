pub mod lexer;
use lexer::token::TokenKind;
use lexer::Lexer;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
struct RoundResult {
    elapsed_secs: f64,
    tokens: usize,
    ended_with_eof: bool,
}

fn build_stress_source(repeat: usize) -> String {
    let mut s = String::with_capacity(repeat * 220);

    s.push_str("fn main(){ let mut total=0; let seed=1; ");

    for i in 0..repeat {
        // ident / number / assign / operator / delimiter / separator
        s.push_str(&format!(
            "let v{i}={i}; total=total+v{i}; total=total&seed; \
             if total>=1000 {{ total=total-999; }} else {{ total=total+1; }} \
             while v{i}<10 {{ v{i}=v{i}+1; }} \
             let arr=[v{i},total,1]; v{i}:total; \
             x->y; x..y; "
        ));

        // keyword coverage
        if i % 32 == 0 {
            s.push_str("for k in 3 { loop { break; } continue; } ");
        }
    }

    s.push_str("return total; }\n#");
    s
}

fn run_one_round(src: &str) -> RoundResult {
    let mut lex = Lexer::new(src);
    let start = Instant::now();

    let mut count: usize = 0;
    let ended_with_eof: bool;

    loop {
        let tok = lex.next_token();
        count += 1;

        match tok.kind {
            TokenKind::Eof => {
                ended_with_eof = true;
                break;
            }
            TokenKind::Error => {
                ended_with_eof = false;
                break;
            }
            _ => {}
        }
    }

    RoundResult {
        elapsed_secs: start.elapsed().as_secs_f64(),
        tokens: count,
        ended_with_eof,
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / (values.len() as f64)
}

fn stddev(values: &[f64], avg: f64) -> f64 {
    let var = values
        .iter()
        .map(|v| {
            let d = *v - avg;
            d * d
        })
        .sum::<f64>()
        / (values.len() as f64);
    var.sqrt()
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    let n = sorted_values.len();
    let idx = ((n as f64 - 1.0) * p).round() as usize;
    sorted_values[idx]
}

fn main() {
    // 可以用环境变量覆盖默认参数：BENCH_REPEAT/BENCH_WARMUP/BENCH_ROUNDS
    let repeat: usize = std::env::var("BENCH_REPEAT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let warmup_rounds: usize = std::env::var("BENCH_WARMUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let measure_rounds: usize = std::env::var("BENCH_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let src = build_stress_source(repeat);
    let bytes = src.len();

    for _ in 0..warmup_rounds {
        let _ = run_one_round(&src);
    }

    let mut results = Vec::with_capacity(measure_rounds);
    for _ in 0..measure_rounds {
        results.push(run_one_round(&src));
    }

    let mut elapsed_list: Vec<f64> = results.iter().map(|r| r.elapsed_secs).collect();
    elapsed_list.sort_by(f64::total_cmp);
    let avg_elapsed = mean(&elapsed_list);
    let p50_elapsed = percentile(&elapsed_list, 0.50);
    let p95_elapsed = percentile(&elapsed_list, 0.95);
    let min_elapsed = elapsed_list[0];
    let max_elapsed = elapsed_list[elapsed_list.len() - 1];
    let sd_elapsed = stddev(&elapsed_list, avg_elapsed);

    let tokens_per_round = results[0].tokens;
    let eof_rounds = results.iter().filter(|r| r.ended_with_eof).count();
    let error_rounds = results.len() - eof_rounds;

    let avg_tps = (tokens_per_round as f64) / avg_elapsed.max(1e-9);
    let p50_tps = (tokens_per_round as f64) / p50_elapsed.max(1e-9);
    let p95_tps = (tokens_per_round as f64) / p95_elapsed.max(1e-9);

    let avg_mib_s = (bytes as f64 / (1024.0 * 1024.0)) / avg_elapsed.max(1e-9);
    let p50_mib_s = (bytes as f64 / (1024.0 * 1024.0)) / p50_elapsed.max(1e-9);
    let p95_mib_s = (bytes as f64 / (1024.0 * 1024.0)) / p95_elapsed.max(1e-9);

    println!("config: repeat={repeat}, warmup_rounds={warmup_rounds}, measure_rounds={measure_rounds}");
    println!("source bytes: {bytes}");
    println!("tokens per round: {tokens_per_round}");
    println!("termination: eof_rounds={eof_rounds}, error_rounds={error_rounds}");
    println!();
    println!("elapsed (s):");
    println!("  min={min_elapsed:.6}, p50={p50_elapsed:.6}, p95={p95_elapsed:.6}, max={max_elapsed:.6}");
    println!("  mean={avg_elapsed:.6}, stddev={sd_elapsed:.6}");
    println!();
    println!("throughput (tokens/s):");
    println!("  p50={p50_tps:.0}, p95={p95_tps:.0}, mean={avg_tps:.0}");
    println!("throughput (MiB/s):");
    println!("  p50={p50_mib_s:.2}, p95={p95_mib_s:.2}, mean={avg_mib_s:.2}");
}