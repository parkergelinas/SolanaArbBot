//! End-to-end decision latency budget — mock Jupiter with 10ms delay.

use std::sync::Arc;
use std::time::{Duration, Instant};

use execution_engine::{
    audit::AuditLog,
    config::EngineConfig,
    jupiter::JupiterExecutor,
    orders::{OrderStatus, OrderStore},
    router::ExecutionRouter,
    signals::TradeSignal,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SOL: &str = "So11111111111111111111111111111111111111112";

fn test_config(jupiter_url: String) -> EngineConfig {
    EngineConfig {
        paper_mode: true,
        execution_live: false,
        min_confidence: 0.55,
        min_edge_bps: 5.0,
        max_position_usd: 500.0,
        default_slippage_bps: 50,
        token_cooldown_secs: 30,
        request_timeout_ms: 800,
        jupiter_base_url: jupiter_url,
        audit_path: None,
        wallet_pubkey: "11111111111111111111111111111111".into(),
        quote_cache_ttl_ms: 200,
        quote_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
        base_mint: SOL.into(),
    }
}

async fn spawn_mock_jupiter(delay_ms: u64) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
    let addr = listener.local_addr().expect("local addr");
    let uri = format!("http://{addr}");

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf).await;
                let body = r#"{"inAmount":"1000000000","outAmount":"145000000","priceImpactPct":"0.01","routePlan":[{"swapInfo":{"label":"Raydium"}}]}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    uri
}

#[tokio::test]
async fn latency_budget_test() {
    let mock_uri = spawn_mock_jupiter(10).await;

    let config = test_config(mock_uri);
    let orders = Arc::new(OrderStore::new());
    let audit = Arc::new(AuditLog::new(None));
    let jupiter = Arc::new(JupiterExecutor::new(config.clone()));
    let router = Arc::new(ExecutionRouter::new(
        config,
        orders.clone(),
        audit,
        jupiter,
        None,
    ));

    let signal = TradeSignal::new(SOL, "long", 0.9, 15.0, 50.0, "momentum_follow");

    let started = Instant::now();
    router.process(signal).await;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    assert!(
        elapsed_ms < 100.0,
        "decision path took {elapsed_ms:.2}ms, budget is 100ms"
    );

    let confirmed = orders
        .all()
        .into_iter()
        .find(|o| o.status == OrderStatus::Confirmed);
    assert!(confirmed.is_some(), "expected confirmed order in paper mode");
}
