//! Jupiter Tokens API v2 quality registry.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use config::DataSourcesConfig;
use tracing::{info, warn};

use crate::jupiter_api::{fetch_verified_tokens, token_passes_quality, JupiterTokenMeta};

#[derive(Clone, Default)]
pub struct TokenQualityFilter {
    inner: Arc<RwLock<TokenQualityInner>>,
}

#[derive(Default)]
struct TokenQualityInner {
    verified: HashSet<String>,
    strict: HashSet<String>,
    loaded: bool,
}

impl TokenQualityFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn refresh(&self, data_sources: &DataSourcesConfig) {
        let client = reqwest::Client::new();
        match fetch_verified_tokens(&client, &data_sources.jupiter_tokens, 5000).await {
            Ok(tokens) => self.ingest(tokens),
            Err(e) => warn!(error = %e, "jupiter tokens v2 refresh failed"),
        }
    }

    pub fn ingest(&self, tokens: Vec<JupiterTokenMeta>) {
        let mut verified = HashSet::new();
        let mut strict = HashSet::new();
        for t in tokens {
            if t.tags.iter().any(|tag| tag.eq_ignore_ascii_case("verified")) {
                verified.insert(t.id.clone());
            }
            if t.tags.iter().any(|tag| tag.eq_ignore_ascii_case("strict")) {
                strict.insert(t.id.clone());
            }
        }
        let mut g = self.inner.write().expect("lock");
        g.verified = verified;
        g.strict = strict;
        g.loaded = true;
        info!(
            verified = g.verified.len(),
            strict = g.strict.len(),
            "jupiter token quality registry updated"
        );
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.read().expect("lock").loaded
    }

    pub fn is_verified(&self, mint: &str) -> bool {
        let g = self.inner.read().expect("lock");
        !g.loaded || g.verified.contains(mint)
    }

    pub fn passes(&self, mint: &str, require_strict: bool) -> bool {
        let g = self.inner.read().expect("lock");
        if !g.loaded {
            return true;
        }
        if require_strict {
            return g.strict.contains(mint);
        }
        g.verified.contains(mint)
    }

    pub fn verified_count(&self) -> usize {
        self.inner.read().expect("lock").verified.len()
    }
}

pub fn meta_passes(meta: &JupiterTokenMeta, require_strict: bool) -> bool {
    token_passes_quality(meta, require_strict)
}
