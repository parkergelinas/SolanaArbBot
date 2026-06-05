//! Control-api process environment: bind address, deploy profile, optional stores.
//!
//! Domain config (`SystemConfig`) lives in the `config` crate. This module covers
//! host-specific settings and production guardrails with actionable startup errors.

use config::SystemConfig;

/// Deployment profile — set `DEPLOY_ENV=production` on Railway/Fly/Render/VPS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeployEnv {
    Development,
    Staging,
    Production,
}

impl DeployEnv {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "development" | "dev" | "local" => Some(Self::Development),
            "staging" | "stage" | "preview" => Some(Self::Staging),
            "production" | "prod" => Some(Self::Production),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }

    pub fn is_strict(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

/// Parsed control-api host/process settings (not `SystemConfig`).
#[derive(Clone, Debug)]
pub struct RuntimeEnv {
    pub host: String,
    pub port: u16,
    pub deploy_env: DeployEnv,
    /// Optional Upstash / Redis URL for future shared signal buffer + trade journal.
    pub redis_url: Option<String>,
}

impl RuntimeEnv {
    /// Load bind address and deploy profile from the process environment.
    pub fn load() -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();

        let host = std::env::var("CONTROL_API_HOST")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_owned());
        if host.trim().is_empty() {
            errors.push("CONTROL_API_HOST must not be empty".to_owned());
        }

        let port = match std::env::var("CONTROL_API_PORT") {
            Ok(raw) => match raw.trim().parse::<u16>() {
                Ok(p) if p > 0 => p,
                _ => {
                    errors.push(format!(
                        "CONTROL_API_PORT must be a valid port 1–65535, got {raw:?}"
                    ));
                    3001
                }
            },
            Err(_) => 3001,
        };

        let deploy_env = match std::env::var("DEPLOY_ENV") {
            Ok(raw) => DeployEnv::parse(&raw).unwrap_or_else(|| {
                errors.push(format!(
                    "DEPLOY_ENV must be development|staging|production, got {raw:?}"
                ));
                DeployEnv::Development
            }),
            Err(_) => DeployEnv::Development,
        };

        let redis_url = std::env::var("UPSTASH_REDIS_REST_URL")
            .or_else(|_| std::env::var("REDIS_URL"))
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());

        if let Some(ref url) = redis_url {
            if !url.starts_with("redis://")
                && !url.starts_with("rediss://")
                && !url.starts_with("https://")
            {
                errors.push(format!(
                    "REDIS_URL / UPSTASH_REDIS_REST_URL must start with redis://, rediss://, or https:// (got {url:?})"
                ));
            }
        }

        if errors.is_empty() {
            Ok(Self {
                host,
                port,
                deploy_env,
                redis_url,
            })
        } else {
            Err(errors)
        }
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Public Solana RPC endpoints bundled as dev defaults — not suitable for production load.
const DEFAULT_RPC_ENDPOINTS: &[&str] = &[
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
];

/// Validate `SystemConfig` + process env for staging/production deploys.
pub fn validate_for_deploy(
    runtime: &RuntimeEnv,
    config: &SystemConfig,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if !runtime.deploy_env.is_strict() {
        return Ok(());
    }

    // Require an explicit paid/private RPC in non-dev environments.
    let rpc_from_env = std::env::var("SOLANA_ARB_RPC__ENDPOINTS").is_ok();
    let only_defaults = config.rpc.endpoints.iter().all(|ep| {
        DEFAULT_RPC_ENDPOINTS
            .iter()
            .any(|d| d.eq_ignore_ascii_case(ep))
    });
    if only_defaults && !rpc_from_env {
        errors.push(
            "production/staging requires a dedicated RPC: set SOLANA_ARB_RPC__ENDPOINTS \
             (comma-separated URLs). Public mainnet endpoints are rate-limited."
                .to_owned(),
        );
    }

    // Live trading requires an explicit keypair source and populated secret.
    if config.features.enable_live_trading && !config.features.dry_run {
        match (
            config.wallet.keypair_env_var.as_deref(),
            config.wallet.keypair_path.as_deref(),
        ) {
            (Some(var), _) => {
                if std::env::var(var)
                    .map(|v| v.trim().is_empty())
                    .unwrap_or(true)
                {
                    errors.push(format!(
                        "live trading enabled but keypair env var {var:?} is unset or empty"
                    ));
                }
            }
            (None, Some(path)) => {
                if !std::path::Path::new(path).exists() {
                    errors.push(format!(
                        "live trading enabled but keypair file not found: {path}"
                    ));
                }
            }
            (None, None) => {
                errors.push(
                    "live trading enabled: set SOLANA_ARB_WALLET__KEYPAIR_ENV_VAR \
                     or SOLANA_ARB_WALLET__KEYPAIR_PATH"
                        .to_owned(),
                );
            }
        }

        if config.wallet.expected_network == "devnet" {
            errors.push(
                "live trading with wallet.expected_network=devnet is not allowed in \
                 production — set SOLANA_ARB_WALLET__EXPECTED_NETWORK=mainnet"
                    .to_owned(),
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Format all startup errors for stderr — one block, easy to grep in host logs.
pub fn format_startup_errors(label: &str, errors: &[String]) -> String {
    let mut out = format!("control-api startup failed — {label}:\n");
    for (i, e) in errors.iter().enumerate() {
        out.push_str(&format!("  {}. {e}\n", i + 1));
    }
    out.push_str("\nSee apps/control-api/.env.example and docs/deploy-control-api.md\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_runtime_env_loads() {
        let rt = RuntimeEnv::load().expect("defaults");
        assert_eq!(rt.port, 3001);
        assert_eq!(rt.deploy_env, DeployEnv::Development);
    }

    #[test]
    fn production_rejects_default_rpc_without_env() {
        let runtime = RuntimeEnv {
            host: "0.0.0.0".to_owned(),
            port: 3001,
            deploy_env: DeployEnv::Production,
            redis_url: None,
        };
        let cfg = SystemConfig::default();
        let err = validate_for_deploy(&runtime, &cfg).expect_err("should fail");
        assert!(err.iter().any(|e| e.contains("SOLANA_ARB_RPC__ENDPOINTS")));
    }
}
