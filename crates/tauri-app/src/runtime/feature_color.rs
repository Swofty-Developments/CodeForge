//! The `set_feature_color` mutation, factored out of the command so it is
//! unit-testable against a real [`FeatureIndex`] without a running Tauri app.
//! Sets `Feature.color` (a `None` clears it), implies a pin (like `update_
//! feature`), persists, and returns the stored feature.

use forge_core::Feature;
use forge_index::FeatureIndex;

/// Set (or clear, with `None`) a feature's graph color and pin it, then save.
/// Returns the persisted feature. An unknown slug is a named error — the color
/// is never applied to a phantom feature (no silent upsert of a new slug).
pub fn set_color(
    index: &mut FeatureIndex,
    slug: &str,
    color: Option<String>,
) -> Result<Feature, String> {
    validate_color(color.as_deref())?;

    let mut feature = index
        .get(slug)
        .cloned()
        .ok_or_else(|| format!("unknown feature: {slug}"))?;
    feature.color = color;
    feature.pinned = true; // setting a color is a human edit → implies pin
    index.upsert(feature);
    index.save().map_err(|e| format!("failed to save feature index: {e}"))?;

    index
        .get(slug)
        .cloned()
        .ok_or_else(|| format!("feature vanished after save: {slug}"))
}

/// A color must be `None` (clear) or a `#RGB` / `#RRGGBB` hex string. Anything
/// else is a named error rather than a silently-stored bad value.
fn validate_color(color: Option<&str>) -> Result<(), String> {
    let Some(c) = color else { return Ok(()) };
    let ok = matches!(c.len(), 4 | 7)
        && c.starts_with('#')
        && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(format!("invalid color {c:?}: expected a hex string like #74ade8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ff-color-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn feature(slug: &str) -> Feature {
        Feature {
            slug: slug.into(),
            name: slug.to_uppercase(),
            description: String::new(),
            entry_points: Vec::new(),
            files: Vec::new(),
            tags: Vec::new(),
            pinned: false,
            color: None,
            group: None,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn set_color_sets_pins_persists_and_reloads() {
        let dir = tmp("set");
        let mut index = FeatureIndex::load(&dir).unwrap();
        index.upsert(feature("auth"));

        let updated = set_color(&mut index, "auth", Some("#74ade8".into())).unwrap();
        assert_eq!(updated.color.as_deref(), Some("#74ade8"));
        assert!(updated.pinned, "setting a color implies a pin");

        // Persisted to disk: a fresh load sees the color + pin.
        let reloaded = FeatureIndex::load(&dir).unwrap();
        let f = reloaded.get("auth").unwrap();
        assert_eq!(f.color.as_deref(), Some("#74ade8"));
        assert!(f.pinned);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn none_clears_the_color() {
        let dir = tmp("clear");
        let mut index = FeatureIndex::load(&dir).unwrap();
        index.upsert(feature("auth"));
        set_color(&mut index, "auth", Some("#fff".into())).unwrap();

        let cleared = set_color(&mut index, "auth", None).unwrap();
        assert_eq!(cleared.color, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_slug_and_bad_color_are_named_errors() {
        let dir = tmp("err");
        let mut index = FeatureIndex::load(&dir).unwrap();
        index.upsert(feature("auth"));

        assert!(set_color(&mut index, "ghost", Some("#fff".into())).unwrap_err().contains("unknown feature"));
        // A phantom slug must not have been created by a stray upsert.
        assert!(index.get("ghost").is_none());
        assert!(set_color(&mut index, "auth", Some("blue".into())).unwrap_err().contains("invalid color"));
        assert!(set_color(&mut index, "auth", Some("#12".into())).unwrap_err().contains("invalid color"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
