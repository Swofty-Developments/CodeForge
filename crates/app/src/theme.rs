use iced::Color;

// t3code-inspired dark palette
// Near-black background with subtle warm undertone
pub const BG_BASE: Color = Color::from_rgb(0.075, 0.075, 0.085); // ~#131316
pub const BG_CARD: Color = Color::from_rgb(0.09, 0.09, 0.10); // ~#171719 — cards, composer
pub const BG_SURFACE: Color = Color::from_rgb(0.105, 0.105, 0.115); // ~#1b1b1e — sidebar
pub const BG_ACCENT: Color = Color::from_rgb(0.14, 0.14, 0.155); // ~#242428 — hover, active items
pub const BG_MUTED: Color = Color::from_rgb(0.12, 0.12, 0.13); // ~#1e1e21 — subtle bg tint
pub const BG_USER_BUBBLE: Color = Color::from_rgb(0.13, 0.13, 0.15); // ~#212126 — user message bg

// Text hierarchy
pub const TEXT: Color = Color::from_rgb(0.92, 0.92, 0.94); // #ebebf0 — primary text
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.55, 0.55, 0.60); // #8c8c99 — muted/secondary
pub const TEXT_TERTIARY: Color = Color::from_rgb(0.38, 0.38, 0.42); // #61616b — faint labels

// Accent colors
pub const PRIMARY: Color = Color::from_rgb(0.40, 0.50, 0.95); // ~#6680f2 — indigo/blue primary
pub const PRIMARY_MUTED: Color = Color::from_rgb(0.35, 0.43, 0.82); // dimmer primary for subtle use

// Semantic
pub const GREEN: Color = Color::from_rgb(0.35, 0.78, 0.55); // #59c78c — success/emerald
pub const RED: Color = Color::from_rgb(0.90, 0.35, 0.38); // #e65961 — destructive
pub const AMBER: Color = Color::from_rgb(0.90, 0.72, 0.30); // #e6b84d — warning/pending
pub const SKY: Color = Color::from_rgb(0.40, 0.72, 0.88); // #66b8e0 — info/working

// Borders — very subtle, like t3code's 6-8% white overlay
pub const BORDER: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.07,
};
pub const BORDER_STRONG: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.12,
};
pub const BORDER_FOCUS: Color = Color {
    r: 0.40,
    g: 0.50,
    b: 0.95,
    a: 0.45,
};

// Thread color palette
pub const THREAD_COLORS: &[(&str, Color)] = &[
    ("#e65961", RED),
    ("#e6b84d", AMBER),
    ("#59c78c", GREEN),
    ("#66b8e0", SKY),
    ("#6680f2", PRIMARY),
    ("#c084fc", Color::from_rgb(0.75, 0.52, 0.99)), // purple
    ("#f472b6", Color::from_rgb(0.96, 0.45, 0.71)), // pink
    ("#fb923c", Color::from_rgb(0.98, 0.57, 0.24)), // orange
];

/// Parse a hex color string like "#e65961" into an iced Color
pub fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

// Radii (as f32 for iced Border)
pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;
pub const RADIUS_XL: f32 = 20.0;
pub const RADIUS_PILL: f32 = 100.0;
