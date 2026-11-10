//! The `buttonVariants` table of `src/components/ui/button.tsx`, resolved against the palette.
//! Sizes are the `sm:` (desktop) columns: the app never runs below the `sm` breakpoint.

use gpui::{black, transparent_black, white, BoxShadow, Hsla};
use quiver_theme::Radius;

use crate::elevation::shadow_xs;
use crate::theme::QuiverTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    DestructiveOutline,
    Ghost,
    Link,
    Outline,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    #[default]
    Default,
    Icon,
    IconLg,
    IconSm,
    IconXl,
    IconXs,
    Lg,
    Sm,
    Xl,
    Xs,
}

/// Box metrics of a size, in px.
#[derive(Debug, Clone, Copy)]
pub struct SizeSpec {
    pub height: f32,
    /// Fixed width for the icon sizes.
    pub width: Option<f32>,
    /// Horizontal padding inside the 1px border (`calc(spacing(n) - 1px)`).
    pub pad_x: f32,
    pub gap: f32,
    pub radius: f32,
    pub text: f32,
    pub icon: f32,
}

impl ButtonSize {
    pub fn spec(self) -> SizeSpec {
        let s = |height, width, pad_x, gap, radius, text, icon| SizeSpec {
            height,
            width,
            pad_x,
            gap,
            radius,
            text,
            icon,
        };
        let (lg, md) = (Radius::LG, Radius::MD);
        match self {
            ButtonSize::Default => s(32., None, 11., 8., lg, 14., 16.),
            ButtonSize::Icon => s(32., Some(32.), 0., 8., lg, 14., 16.),
            ButtonSize::IconLg => s(36., Some(36.), 0., 8., lg, 14., 16.),
            ButtonSize::IconSm => s(28., Some(28.), 0., 8., lg, 14., 16.),
            ButtonSize::IconXl => s(40., Some(40.), 0., 8., lg, 14., 18.),
            ButtonSize::IconXs => s(24., Some(24.), 0., 8., md, 14., 14.),
            ButtonSize::Lg => s(36., None, 13., 8., lg, 14., 16.),
            ButtonSize::Sm => s(28., None, 9., 6., lg, 14., 16.),
            ButtonSize::Xl => s(40., None, 15., 8., lg, 16., 18.),
            ButtonSize::Xs => s(24., None, 7., 4., md, 12., 14.),
        }
    }
}

/// Resolved colours of a variant in the current mode.
#[derive(Debug, Clone)]
pub struct VariantStyle {
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
    pub hover_bg: Hsla,
    pub hover_border: Option<Hsla>,
    pub shadow: Vec<BoxShadow>,
    pub underline_on_hover: bool,
}

impl ButtonVariant {
    pub fn resolve(
        self,
        q: &QuiverTheme,
    ) -> VariantStyle {
        let p = &q.palette;
        let dark = q.is_dark();
        let none = transparent_black();
        let outline_bg = if dark {
            p.input.opacity(0.32)
        } else {
            p.popover
        };
        let base = VariantStyle {
            bg: none,
            fg: p.foreground,
            border: none,
            hover_bg: none,
            hover_border: None,
            shadow: vec![],
            underline_on_hover: false,
        };
        match self {
            ButtonVariant::Default => VariantStyle {
                bg: p.primary,
                fg: p.primary_foreground,
                border: p.primary,
                hover_bg: p.primary.opacity(0.9),
                shadow: shadow_xs(p.primary.opacity(0.24)),
                ..base
            },
            ButtonVariant::Destructive => VariantStyle {
                bg: p.destructive,
                fg: white(),
                border: p.destructive,
                hover_bg: p.destructive.opacity(0.9),
                shadow: shadow_xs(p.destructive.opacity(0.24)),
                ..base
            },
            ButtonVariant::DestructiveOutline => VariantStyle {
                bg: outline_bg,
                fg: p.destructive,
                border: p.input,
                hover_bg: p.destructive.opacity(0.04),
                hover_border: Some(p.destructive.opacity(0.32)),
                shadow: shadow_xs(black().opacity(0.05)),
                ..base
            },
            ButtonVariant::Ghost => VariantStyle {
                hover_bg: p.accent,
                ..base
            },
            ButtonVariant::Link => VariantStyle {
                underline_on_hover: true,
                ..base
            },
            ButtonVariant::Outline => VariantStyle {
                bg: outline_bg,
                border: p.input,
                hover_bg: if dark {
                    p.input.opacity(0.64)
                } else {
                    p.accent.opacity(0.5)
                },
                shadow: shadow_xs(black().opacity(0.05)),
                ..base
            },
            ButtonVariant::Secondary => VariantStyle {
                bg: p.secondary,
                fg: p.secondary_foreground,
                hover_bg: p.secondary.opacity(0.9),
                ..base
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_heights_match_the_sm_breakpoint() {
        assert_eq!(ButtonSize::Default.spec().height, 32.0);
        assert_eq!(ButtonSize::Sm.spec().height, 28.0);
        assert_eq!(ButtonSize::Xs.spec().height, 24.0);
        assert_eq!(ButtonSize::IconXs.spec().width, Some(24.0));
    }
}
