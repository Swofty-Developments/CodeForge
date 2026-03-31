//! Color types with hex parsing, HSL conversion, RGBA, and contrast ratio.
//!
//! Provides a unified [`Color`] type for use in theme definitions, syntax
//! highlighting, and UI rendering. Supports conversions between RGB, HSL,
//! and hex string representations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// An RGBA color with 8-bit components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    /// Red channel (0-255).
    pub r: u8,
    /// Green channel (0-255).
    pub g: u8,
    /// Blue channel (0-255).
    pub b: u8,
    /// Alpha channel (0-255, where 255 is fully opaque).
    pub a: u8,
}

impl Color {
    /// Create a fully opaque RGB color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create an RGBA color.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from a hex string like "#ff00ff" or "#ff00ff80".
    pub fn from_hex(hex: &str) -> Result<Self, ColorParseError> {
        hex.parse()
    }

    /// Convert to a 6-digit hex string like "#ff00ff".
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Convert to an 8-digit hex string like "#ff00ff80".
    pub fn to_hex_rgba(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// Convert to an HSL representation.
    pub fn to_hsl(&self) -> Hsl {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let lightness = (max + min) / 2.0;

        if delta < f64::EPSILON {
            return Hsl {
                h: 0.0,
                s: 0.0,
                l: lightness,
            };
        }

        let saturation = if lightness < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let hue = if (max - r).abs() < f64::EPSILON {
            ((g - b) / delta) % 6.0
        } else if (max - g).abs() < f64::EPSILON {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };

        let hue = hue * 60.0;
        let hue = if hue < 0.0 { hue + 360.0 } else { hue };

        Hsl {
            h: hue,
            s: saturation,
            l: lightness,
        }
    }

    /// Create a color from HSL values.
    pub fn from_hsl(hsl: &Hsl) -> Self {
        if hsl.s < f64::EPSILON {
            let v = (hsl.l * 255.0).round() as u8;
            return Self::rgb(v, v, v);
        }

        let q = if hsl.l < 0.5 {
            hsl.l * (1.0 + hsl.s)
        } else {
            hsl.l + hsl.s - hsl.l * hsl.s
        };
        let p = 2.0 * hsl.l - q;
        let h = hsl.h / 360.0;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        Self::rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    }

    /// Return the relative luminance per WCAG 2.1.
    pub fn relative_luminance(&self) -> f64 {
        let convert = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * convert(self.r) + 0.7152 * convert(self.g) + 0.0722 * convert(self.b)
    }

    /// Calculate the WCAG contrast ratio between two colors.
    /// Returns a value between 1.0 and 21.0.
    pub fn contrast_ratio(&self, other: &Color) -> f64 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check whether text of this color on a background meets WCAG AA (4.5:1).
    pub fn meets_wcag_aa(&self, background: &Color) -> bool {
        self.contrast_ratio(background) >= 4.5
    }

    /// Check whether text of this color on a background meets WCAG AAA (7:1).
    pub fn meets_wcag_aaa(&self, background: &Color) -> bool {
        self.contrast_ratio(background) >= 7.0
    }

    /// Blend this color with another using the given alpha (0.0-1.0).
    pub fn blend(&self, other: &Color, alpha: f64) -> Color {
        let alpha = alpha.clamp(0.0, 1.0);
        let inv = 1.0 - alpha;
        Color::rgb(
            (self.r as f64 * inv + other.r as f64 * alpha).round() as u8,
            (self.g as f64 * inv + other.g as f64 * alpha).round() as u8,
            (self.b as f64 * inv + other.b as f64 * alpha).round() as u8,
        )
    }

    /// Lighten the color by a percentage (0.0-1.0).
    pub fn lighten(&self, amount: f64) -> Color {
        let mut hsl = self.to_hsl();
        hsl.l = (hsl.l + amount).min(1.0);
        Color::from_hsl(&hsl)
    }

    /// Darken the color by a percentage (0.0-1.0).
    pub fn darken(&self, amount: f64) -> Color {
        let mut hsl = self.to_hsl();
        hsl.l = (hsl.l - amount).max(0.0);
        Color::from_hsl(&hsl)
    }

    /// Generate a CSS `rgba()` string.
    pub fn to_css_rgba(&self) -> String {
        if self.a == 255 {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            let a = self.a as f64 / 255.0;
            format!("rgba({}, {}, {}, {:.2})", self.r, self.g, self.b, a)
        }
    }

    // Common color constants.

    /// Black (#000000).
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White (#ffffff).
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Transparent black.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
}

/// HSL color representation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hsl {
    /// Hue in degrees (0-360).
    pub h: f64,
    /// Saturation (0.0-1.0).
    pub s: f64,
    /// Lightness (0.0-1.0).
    pub l: f64,
}

impl fmt::Display for Hsl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hsl({:.0}, {:.0}%, {:.0}%)",
            self.h,
            self.s * 100.0,
            self.l * 100.0
        )
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Error returned when parsing a color string fails.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid color: {reason}")]
pub struct ColorParseError {
    /// Explanation of why the parse failed.
    pub reason: String,
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid red: {e}"),
                })?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid green: {e}"),
                })?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid blue: {e}"),
                })?;
                Ok(Color::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid red: {e}"),
                })?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid green: {e}"),
                })?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid blue: {e}"),
                })?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid alpha: {e}"),
                })?;
                Ok(Color::rgba(r, g, b, a))
            }
            3 => {
                // Shorthand like "fff"
                let r = u8::from_str_radix(&hex[0..1], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid red: {e}"),
                })?;
                let g = u8::from_str_radix(&hex[1..2], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid green: {e}"),
                })?;
                let b = u8::from_str_radix(&hex[2..3], 16).map_err(|e| ColorParseError {
                    reason: format!("invalid blue: {e}"),
                })?;
                Ok(Color::rgb(r * 17, g * 17, b * 17))
            }
            _ => Err(ColorParseError {
                reason: format!("expected 3, 6, or 8 hex characters, got {}", hex.len()),
            }),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A theme color palette for the application UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePalette {
    /// Primary brand color.
    pub primary: Color,
    /// Secondary accent color.
    pub secondary: Color,
    /// Background color.
    pub background: Color,
    /// Surface color (cards, panels).
    pub surface: Color,
    /// Primary text color.
    pub text: Color,
    /// Muted/secondary text color.
    pub text_muted: Color,
    /// Border/divider color.
    pub border: Color,
    /// Success indicator color.
    pub success: Color,
    /// Warning indicator color.
    pub warning: Color,
    /// Error/danger color.
    pub error: Color,
    /// Info color.
    pub info: Color,
}

impl ThemePalette {
    /// Generate CSS custom property declarations for this palette.
    pub fn to_css_variables(&self) -> String {
        let mut css = String::new();
        css.push_str(&format!("  --color-primary: {};\n", self.primary.to_hex()));
        css.push_str(&format!(
            "  --color-secondary: {};\n",
            self.secondary.to_hex()
        ));
        css.push_str(&format!(
            "  --color-background: {};\n",
            self.background.to_hex()
        ));
        css.push_str(&format!(
            "  --color-surface: {};\n",
            self.surface.to_hex()
        ));
        css.push_str(&format!("  --color-text: {};\n", self.text.to_hex()));
        css.push_str(&format!(
            "  --color-text-muted: {};\n",
            self.text_muted.to_hex()
        ));
        css.push_str(&format!("  --color-border: {};\n", self.border.to_hex()));
        css.push_str(&format!(
            "  --color-success: {};\n",
            self.success.to_hex()
        ));
        css.push_str(&format!(
            "  --color-warning: {};\n",
            self.warning.to_hex()
        ));
        css.push_str(&format!("  --color-error: {};\n", self.error.to_hex()));
        css.push_str(&format!("  --color-info: {};\n", self.info.to_hex()));
        css
    }

    /// Return the default dark theme palette.
    pub fn dark() -> Self {
        Self {
            primary: Color::from_hex("#6366f1").unwrap(),
            secondary: Color::from_hex("#8b5cf6").unwrap(),
            background: Color::from_hex("#0f172a").unwrap(),
            surface: Color::from_hex("#1e293b").unwrap(),
            text: Color::from_hex("#f8fafc").unwrap(),
            text_muted: Color::from_hex("#94a3b8").unwrap(),
            border: Color::from_hex("#334155").unwrap(),
            success: Color::from_hex("#22c55e").unwrap(),
            warning: Color::from_hex("#f59e0b").unwrap(),
            error: Color::from_hex("#ef4444").unwrap(),
            info: Color::from_hex("#3b82f6").unwrap(),
        }
    }

    /// Return the default light theme palette.
    pub fn light() -> Self {
        Self {
            primary: Color::from_hex("#4f46e5").unwrap(),
            secondary: Color::from_hex("#7c3aed").unwrap(),
            background: Color::from_hex("#ffffff").unwrap(),
            surface: Color::from_hex("#f8fafc").unwrap(),
            text: Color::from_hex("#0f172a").unwrap(),
            text_muted: Color::from_hex("#64748b").unwrap(),
            border: Color::from_hex("#e2e8f0").unwrap(),
            success: Color::from_hex("#16a34a").unwrap(),
            warning: Color::from_hex("#d97706").unwrap(),
            error: Color::from_hex("#dc2626").unwrap(),
            info: Color::from_hex("#2563eb").unwrap(),
        }
    }

    /// Validate that all text colors have sufficient contrast against backgrounds.
    pub fn validate_contrast(&self) -> Vec<ContrastIssue> {
        let mut issues = Vec::new();
        let check = |name: &str, fg: &Color, bg: &Color, issues: &mut Vec<ContrastIssue>| {
            let ratio = fg.contrast_ratio(bg);
            if ratio < 4.5 {
                issues.push(ContrastIssue {
                    foreground: name.to_string(),
                    ratio,
                    required: 4.5,
                });
            }
        };
        check("text on background", &self.text, &self.background, &mut issues);
        check("text_muted on background", &self.text_muted, &self.background, &mut issues);
        check("text on surface", &self.text, &self.surface, &mut issues);
        check("text_muted on surface", &self.text_muted, &self.surface, &mut issues);
        issues
    }
}

/// A contrast ratio violation found during palette validation.
#[derive(Debug, Clone)]
pub struct ContrastIssue {
    /// Description of the color pair.
    pub foreground: String,
    /// The actual contrast ratio.
    pub ratio: f64,
    /// The minimum required ratio.
    pub required: f64,
}

impl fmt::Display for ContrastIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: contrast ratio {:.2} below required {:.1}",
            self.foreground, self.ratio, self.required
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Color::rgb(255, 128, 0);
        let hex = c.to_hex();
        let parsed: Color = hex.parse().unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn contrast_ratio_black_white() {
        let ratio = Color::BLACK.contrast_ratio(&Color::WHITE);
        assert!((ratio - 21.0).abs() < 0.1);
    }

    #[test]
    fn hsl_roundtrip() {
        let c = Color::rgb(100, 150, 200);
        let hsl = c.to_hsl();
        let back = Color::from_hsl(&hsl);
        assert!((c.r as i16 - back.r as i16).unsigned_abs() <= 1);
        assert!((c.g as i16 - back.g as i16).unsigned_abs() <= 1);
        assert!((c.b as i16 - back.b as i16).unsigned_abs() <= 1);
    }

    #[test]
    fn dark_palette_contrast() {
        let palette = ThemePalette::dark();
        let issues = palette.validate_contrast();
        // Our default dark palette should have good contrast.
        assert!(issues.is_empty(), "contrast issues: {:?}", issues.iter().map(|i| i.to_string()).collect::<Vec<_>>());
    }
}
