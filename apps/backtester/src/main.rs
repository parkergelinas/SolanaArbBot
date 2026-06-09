//! Dual-strategy backtest CLI: scalping + DEX-to-DEX arbitrage.
//!
//! ```bash
//! cargo run -p backtester-app -- --hours 24
//! cargo run -p backtester-app -- --hours 6 --interval 10 --output data/backtest_results.json
//! ```

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use backtester::{
    generate_dataset, load_from_csv, run_pipeline, strategy_config, PipelineReport,
};
use config::ConfigHandle;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "error".into()),
        )
        .init();

    let cli = Cli::parse();

    let cfg = if let Some(path) = &cli.config {
        ConfigHandle::load_from_file(path)
            .expect("failed to load config")
            .arc()
    } else {
        Arc::new(strategy_config())
    };

    let dataset = if let Some(ref csv_path) = cli.dataset_path {
        info!(path = %csv_path.display(), "loading dataset from CSV");
        load_from_csv(csv_path.to_str().unwrap_or(""))
            .expect("failed to load dataset from CSV")
    } else {
        let duration_secs = cli.hours * 3600;
        info!(
            hours = cli.hours,
            interval_secs = cli.interval,
            events_est = duration_secs / cli.interval,
            "generating synthetic replay dataset"
        );
        generate_dataset(duration_secs, cli.interval)
    };
    info!(
        events = dataset.events.len(),
        pools = dataset.pool_count,
        "dataset ready — starting pipeline"
    );

    let report = run_pipeline(cfg, &dataset, cli.train_fraction);

    print_report(&report);

    if let Some(path) = &cli.output {
        write_report(path, &report);
        info!(path = %path.display(), "results written");

        let dashboard_public = PathBuf::from("apps/dashboard/public/data/backtest_results.json");
        if dashboard_public.parent().is_some() {
            if let Some(parent) = dashboard_public.parent() {
                fs::create_dir_all(parent).ok();
            }
            if fs::copy(path, &dashboard_public).is_ok() {
                info!(path = %dashboard_public.display(), "dashboard copy written");
            }
        }

        let cfg_path = path
            .parent()
            .map(|p| p.join("optimized_config.toml"))
            .unwrap_or_else(|| PathBuf::from("data/optimized_config.toml"));
        if let Ok(toml) = toml::to_string_pretty(&report.recommended_config) {
            if fs::write(&cfg_path, toml).is_ok() {
                info!(path = %cfg_path.display(), "optimized config written");
            }
        }
    }
}

fn print_report(report: &PipelineReport) {
    let b = &report.baseline.metrics;
    let r = &report.retest.metrics;
    let s = &report.simulation;

    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("  DUAL-STRATEGY BACKTEST PIPELINE");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("── Phase 1: Baseline Backtest (train) ──");
    print_metrics(b);
    println!(
        "  Verification: {}",
        if report.baseline_verification.passed {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!();
    println!("── Phase 2: Parameter Optimization ──");
    println!("  Candidates evaluated: {}", report.optimization.candidates_evaluated);
    println!("  Best score: {:.2}", report.optimization.best_score);
    println!(
        "  Best params: edge={}bps cooldown={}s size=${:.0} profit_min=${:.2}",
        report.optimization.best.min_edge_bps,
        report.optimization.best.trade_cooldown_secs,
        report.optimization.best.simulation_initial_amount_usd,
        report.optimization.best.min_profit_threshold_usd,
    );
    println!();
    println!("── Phase 3: Retest (holdout) ──");
    print_metrics(r);
    println!(
        "  Verification: {}",
        if report.retest_verification.passed {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!();
    println!("── Phase 4: Forward Simulation ──");
    println!("  Net PnL:        ${:.2}", s.net_pnl_usd);
    println!("  Trades:         {}", s.trades_executed);
    println!("  Trades/hour:    {:.1}", s.trades_per_hour);
    println!("  Avg net/trade:  ${:.3}", s.avg_net_per_trade_usd);
    println!("  Win rate:       {:.1}%", s.win_rate * 100.0);
    println!("  Scalp PnL:      ${:.2}", s.scalp_pnl_usd);
    println!("  Arb PnL:        ${:.2}", s.arb_pnl_usd);
    println!();
    println!("═══════════════════════════════════════════════════════════");
}

fn print_metrics(m: &backtester::BacktestMetrics) {
    println!("  Combined net:   ${:.2}", m.combined_net_pnl_usd);
    println!("  Combined trades/day: {:.0}", m.combined_trades_per_day);
    println!("  Scalp: {} trades, ${:.2} net, {:.1}% win",
        m.scalp.winning_trades + m.scalp.losing_trades,
        m.scalp.net_pnl_usd,
        m.scalp.win_rate * 100.0,
    );
    println!("  Arb:   {} trades, ${:.2} net, {:.1}% win",
        m.arb.winning_trades + m.arb.losing_trades,
        m.arb.net_pnl_usd,
        m.arb.win_rate * 100.0,
    );
}

fn write_report(path: &PathBuf, report: &PipelineReport) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(report).expect("serialize report");
    fs::write(path, json).expect("write report");
}

struct Cli {
    hours: u64,
    interval: u64,
    train_fraction: f64,
    config: Option<PathBuf>,
    output: Option<PathBuf>,
    /// When set, load events from a CSV file instead of generating synthetic data.
    dataset_path: Option<PathBuf>,
}

impl Cli {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut hours = 6u64;
        let mut interval = 10u64;
        let mut train_fraction = 0.80;
        let mut config = None;
        let mut output = Some(PathBuf::from("data/backtest_results.json"));
        let mut dataset_path: Option<PathBuf> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--hours" | "-h" if i + 1 < args.len() => {
                    hours = args[i + 1].parse().unwrap_or(6);
                    i += 2;
                }
                "--interval" if i + 1 < args.len() => {
                    interval = args[i + 1].parse().unwrap_or(10);
                    i += 2;
                }
                "--train-fraction" if i + 1 < args.len() => {
                    train_fraction = args[i + 1].parse().unwrap_or(0.8);
                    i += 2;
                }
                "--config" | "-c" if i + 1 < args.len() => {
                    config = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "--output" | "-o" if i + 1 < args.len() => {
                    output = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "--dataset-path" | "-d" if i + 1 < args.len() => {
                    dataset_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                }
                "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                other => {
                    eprintln!("unknown argument: {other}");
                    print_help();
                    std::process::exit(1);
                }
            }
        }

        Self {
            hours,
            interval,
            train_fraction,
            config,
            output,
            dataset_path,
        }
    }
}

fn print_help() {
    println!(
        "Usage: backtester [OPTIONS]

Options:
  --hours <N>            Simulated hours of market data (default: 6)
  --interval <SEC>       Seconds between events (default: 10)
  --train-fraction <F>   Train/holdout split (default: 0.80)
  --config <PATH>        Optional config.toml (default: strategy_config)
  --output <PATH>        JSON output path (default: data/backtest_results.json)
  --dataset-path <PATH>  Load events from CSV instead of generating synthetic data
  --help                 Show this help
"
    );
}
