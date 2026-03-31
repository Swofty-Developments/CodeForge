//! Plugin marketplace types for discovery and installation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::manifest::PluginManifest;

/// A source from which plugins can be discovered and installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSource {
    /// Unique identifier for this source.
    pub id: String,
    /// Human-readable source name.
    pub name: String,
    /// Base URL for the marketplace API.
    pub url: String,
    /// Whether this source is enabled.
    pub enabled: bool,
    /// Whether this is a trusted first-party source.
    pub trusted: bool,
}

impl MarketplaceSource {
    /// Create a new marketplace source.
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: url.into(),
            enabled: true,
            trusted: false,
        }
    }

    /// Mark this source as trusted.
    pub fn trusted(mut self) -> Self {
        self.trusted = true;
        self
    }
}

/// A plugin listing in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginListing {
    /// The plugin's manifest.
    pub manifest: PluginManifest,
    /// The marketplace source this listing came from.
    pub source_id: String,
    /// Total number of downloads.
    pub downloads: u64,
    /// Average rating (0.0 to 5.0).
    pub rating: f32,
    /// Number of ratings.
    pub rating_count: u32,
    /// When the plugin was first published.
    pub published_at: DateTime<Utc>,
    /// When the plugin was last updated.
    pub updated_at: DateTime<Utc>,
    /// Whether the plugin has been verified by the marketplace.
    pub verified: bool,
    /// Download URL for the plugin package.
    pub download_url: String,
    /// SHA-256 hash of the plugin package for verification.
    pub checksum: Option<String>,
}

impl PluginListing {
    /// Returns `true` if the plugin is popular (>1000 downloads and >4.0 rating).
    pub fn is_popular(&self) -> bool {
        self.downloads > 1000 && self.rating > 4.0
    }

    /// Returns `true` if the plugin is from a trusted source and has been verified.
    pub fn is_trustworthy(&self) -> bool {
        self.verified
    }

    /// Returns the age of the plugin since its last update.
    pub fn age_since_update(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.updated_at)
    }
}

impl std::fmt::Display for PluginListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} v{} by {} ({} downloads, {:.1} stars)",
            self.manifest.name,
            self.manifest.version,
            self.manifest.author.name,
            self.downloads,
            self.rating
        )
    }
}

/// Sort options for marketplace search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    /// Sort by relevance to search query.
    Relevance,
    /// Sort by download count (most popular first).
    Downloads,
    /// Sort by rating (highest rated first).
    Rating,
    /// Sort by update date (most recent first).
    RecentlyUpdated,
    /// Sort by publish date (newest first).
    Newest,
}

/// Search parameters for the marketplace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Text query to search for.
    pub query: Option<String>,
    /// Filter by tags.
    pub tags: Vec<String>,
    /// Sort order.
    pub sort_by: Option<SortBy>,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}
