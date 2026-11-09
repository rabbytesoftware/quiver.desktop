//! Radii, spacing, type and fixed layout measurements (`:root` and `@theme inline`).

/// `--radius: 10px` and the scale derived from it.
#[derive(Debug, Clone, Copy)]
pub struct Radius;

impl Radius {
    pub const BASE: f32 = 10.0;
    pub const SM: f32 = Self::BASE - 4.0;
    pub const MD: f32 = Self::BASE - 2.0;
    pub const LG: f32 = Self::BASE;
    pub const XL: f32 = Self::BASE + 4.0;
    pub const XXL: f32 = Self::BASE + 8.0;
}

/// Fixed layout measurements in logical pixels.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    /// `--rail`: sidebar width.
    pub rail: f32,
    /// `--row`: sidebar row height.
    pub row: f32,
    /// `--arrow-app-header`.
    pub arrow_app_header: f32,
    /// `--icon`.
    pub icon: f32,
    /// `--spacing` unit (Tailwind `1` = 4px).
    pub spacing: f32,
}

impl Layout {
    /// `--inset: (row - icon) / 2`.
    pub const fn inset(&self) -> f32 {
        (self.row - self.icon) / 2.0
    }
}

pub const LAYOUT: Layout = Layout { rail: 246.0, row: 34.0, arrow_app_header: 44.0, icon: 20.0, spacing: 4.0 };

/// Font families (`--font-sans`, `--font-mono`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    Sans,
    Mono,
}

impl FontFamily {
    pub const fn family(self) -> &'static str {
        match self {
            FontFamily::Sans => "Inter",
            FontFamily::Mono => "JetBrains Mono",
        }
    }
}

/// Body is 13px; html 16px (`@layer base`).
#[derive(Debug, Clone, Copy)]
pub struct TypeScale;

impl TypeScale {
    pub const BODY: f32 = 13.0;
    pub const ROOT: f32 = 16.0;
}
