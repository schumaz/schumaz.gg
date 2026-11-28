use super::analytics::{OPGG_MCP_ENDPOINT, OPGG_REST_API_BASE};
use crate::models::{
    AppLocale, FlashKeyPreference, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, canonical_build_queue_id,
};
use std::path::{Path, PathBuf};

/// Parameter object encapsulating filters, endpoints, preferences, and cache directories
/// for resolving a champion build without cascading positional arguments.
#[derive(Debug, Clone)]
pub struct BuildQuery<'a> {
    pub champion_id: i32,
    pub queue_id: i32,
    pub position: Option<&'a str>,
    pub tier: Option<&'a str>,
    pub region: Option<&'a str>,
    pub patch: Option<&'a str>,
    pub rest_base: &'a str,
    pub mcp_endpoint: &'a str,
    pub patch_48h_protection: bool,
    pub locale: AppLocale,
    pub flash_key: FlashKeyPreference,
    pub builds_dir: Option<&'a Path>,
}

impl<'a> BuildQuery<'a> {
    pub fn new(champion_id: i32, queue_id: i32) -> Self {
        let prefs = crate::storage::get_user_preferences();
        let builds_dir = crate::storage::global_paths().map(|p| p.opgg_builds_dir.as_path());
        Self {
            champion_id,
            queue_id: canonical_build_queue_id(queue_id),
            position: None,
            tier: None,
            region: None,
            patch: None,
            rest_base: OPGG_REST_API_BASE,
            mcp_endpoint: OPGG_MCP_ENDPOINT,
            patch_48h_protection: prefs.patch_48h_protection,
            locale: prefs.language,
            flash_key: prefs.flash_key,
            builds_dir,
        }
    }

    pub fn with_filters(
        mut self,
        position: Option<&'a str>,
        tier: Option<&'a str>,
        region: Option<&'a str>,
        patch: Option<&'a str>,
    ) -> Self {
        self.position = position;
        self.tier = tier;
        self.region = region;
        self.patch = patch;
        self
    }

    pub fn with_endpoints(mut self, rest_base: &'a str, mcp_endpoint: &'a str) -> Self {
        self.rest_base = rest_base;
        self.mcp_endpoint = mcp_endpoint;
        if rest_base != OPGG_REST_API_BASE {
            self.builds_dir = None;
        }
        self
    }

    pub fn with_patch_48h_protection(mut self, enabled: bool) -> Self {
        self.patch_48h_protection = enabled;
        self
    }

    pub fn with_locale(mut self, locale: AppLocale) -> Self {
        self.locale = locale;
        self
    }

    pub fn with_flash_key(mut self, flash_key: FlashKeyPreference) -> Self {
        self.flash_key = flash_key;
        self
    }

    pub fn with_builds_dir(mut self, dir: Option<&'a Path>) -> Self {
        self.builds_dir = dir;
        self
    }

    pub fn canonical_queue_id(&self) -> i32 {
        canonical_build_queue_id(self.queue_id)
    }

    pub fn is_aram(&self) -> bool {
        let q = self.canonical_queue_id();
        q == QUEUE_ARAM || q == QUEUE_ARAM_MAYHEM
    }

    pub fn target_region(&self) -> &str {
        self.region
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("global")
    }

    pub fn target_tier(&self) -> &str {
        match self.tier.map(str::trim).filter(|t| !t.is_empty()) {
            Some(explicit) => explicit,
            None if self.is_aram() => "all",
            None => "emerald_plus",
        }
    }

    pub fn pos_cache_key(&self) -> &str {
        if self.is_aram() {
            ""
        } else {
            self.position.unwrap_or("").trim()
        }
    }

    pub fn manual_patch_trimmed(&self) -> &str {
        self.patch.unwrap_or("").trim()
    }

    pub fn cache_key(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.locale.as_str(),
            self.rest_base,
            self.mcp_endpoint,
            self.champion_id,
            self.canonical_queue_id(),
            self.pos_cache_key(),
            self.target_tier(),
            self.target_region(),
            self.manual_patch_trimmed(),
            self.patch_48h_protection
        )
    }

    pub fn cache_file_path(&self, dir: &Path) -> PathBuf {
        super::cache::build_cache_file_path_for(dir, self)
    }

    pub fn arena_dir(&self) -> Option<&Path> {
        if self.rest_base == OPGG_REST_API_BASE {
            crate::storage::global_paths()
                .map(|p| p.opgg_arena_dir.as_path())
                .or(self.builds_dir)
        } else {
            self.builds_dir
        }
    }
}

/// Request parameters for OP.GG REST champion analysis calls.
#[derive(Debug, Clone, Copy)]
pub struct RestAnalysisRequest<'a> {
    pub rest_base: &'a str,
    pub region: &'a str,
    pub mode: &'a str,
    pub champion_id: i32,
    pub position: Option<&'a str>,
    pub tier: &'a str,
    pub version: Option<&'a str>,
}
