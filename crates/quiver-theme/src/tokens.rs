//! The colour tokens of `:root` (light) and `.dark`.

use crate::color::{Color, Oklch};
use crate::palette::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

macro_rules! tokens {
    ($($field:ident),* $(,)?) => {
        /// Every semantic colour of the design system, resolved to sRGB.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct Tokens {
            $(pub $field: Color,)*
        }
    };
}

tokens!(
    background,
    foreground,
    foreground_subtle,
    card,
    card_foreground,
    popover,
    popover_foreground,
    primary,
    primary_foreground,
    secondary,
    secondary_foreground,
    muted,
    muted_foreground,
    accent,
    accent_foreground,
    destructive,
    destructive_foreground,
    info,
    info_foreground,
    success,
    success_foreground,
    warning,
    warning_foreground,
    border,
    input,
    ring,
    chart_1,
    chart_2,
    chart_3,
    chart_4,
    chart_5,
    sidebar,
    sidebar_foreground,
    sidebar_primary,
    sidebar_primary_foreground,
    sidebar_accent,
    sidebar_accent_foreground,
    sidebar_border,
    sidebar_ring,
    selected_edge,
    sidebar_element_idle,
    sidebar_element_hover,
    sidebar_count_added,
    sidebar_count_deleted,
    console,
    console_foreground,
    console_dim,
    console_row_hover,
    console_row_open,
    log_info,
    log_debug,
    log_warn,
    log_error,
    log_warn_text,
    log_error_text,
    log_number,
    log_bool,
    log_path,
    log_field_error,
    log_underline,
);

fn g(l: f32) -> Color {
    Oklch::new(l, 0.0, 0.0).to_color()
}

fn ga(
    l: f32,
    a: f32,
) -> Color {
    Oklch::alpha(l, 0.0, 0.0, a).to_color()
}

impl Tokens {
    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self::light(),
            Mode::Dark => Self::dark(),
        }
    }

    fn light() -> Self {
        let background = g(1.0);
        let foreground = g(0.145);
        Self {
            background,
            foreground,
            foreground_subtle: g(0.62),
            card: g(1.0),
            card_foreground: g(0.18),
            popover: g(1.0),
            popover_foreground: g(0.18),
            primary: g(0.18),
            primary_foreground: g(0.98),
            secondary: g(0.94),
            secondary_foreground: g(0.18),
            muted: g(0.94),
            muted_foreground: g(0.633),
            accent: g(0.94),
            accent_foreground: g(0.18),
            destructive: RED_300.to_color().mix_srgb(0.6, Color::BLACK),
            destructive_foreground: RED_700.to_color(),
            info: BLUE_500.to_color(),
            info_foreground: BLUE_700.to_color(),
            success: EMERALD_500.to_color(),
            success_foreground: EMERALD_700.to_color(),
            warning: AMBER_500.to_color(),
            warning_foreground: AMBER_700.to_color(),
            border: g(0.919),
            input: g(0.94),
            ring: g(0.5),
            chart_1: g(0.3),
            chart_2: g(0.42),
            chart_3: g(0.54),
            chart_4: g(0.66),
            chart_5: g(0.78),
            sidebar: g(0.951),
            sidebar_foreground: g(0.218),
            sidebar_primary: g(0.145),
            sidebar_primary_foreground: g(1.0),
            sidebar_accent: g(0.97),
            sidebar_accent_foreground: g(0.218),
            sidebar_border: g(0.919),
            sidebar_ring: g(0.5),
            selected_edge: background.with_alpha(0.16),
            sidebar_element_idle: foreground.with_alpha(0.06),
            sidebar_element_hover: ga(0.0, 0.08),
            sidebar_count_added: GREEN_300.to_color().mix_srgb(0.6, Color::BLACK),
            sidebar_count_deleted: RED_300.to_color().mix_srgb(0.6, Color::BLACK),
            console: g(0.975),
            console_foreground: g(0.18),
            console_dim: g(0.5),
            console_row_hover: ga(0.0, 0.04),
            console_row_open: ga(0.0, 0.06),
            log_info: g(0.35),
            log_debug: g(0.55),
            log_warn: Oklch::new(0.5, 0.12, 70.0).to_color(),
            log_error: Oklch::new(0.5, 0.19, 25.0).to_color(),
            log_warn_text: Oklch::new(0.42, 0.1, 70.0).to_color(),
            log_error_text: Oklch::new(0.42, 0.15, 25.0).to_color(),
            log_number: Oklch::new(0.48, 0.15, 255.0).to_color(),
            log_bool: Oklch::new(0.45, 0.12, 160.0).to_color(),
            log_path: Oklch::new(0.4, 0.03, 90.0).to_color(),
            log_field_error: Oklch::new(0.48, 0.17, 25.0).to_color(),
            log_underline: ga(0.0, 0.25),
        }
    }

    fn dark() -> Self {
        let mut t = Self::light();
        let background = g(0.178);
        t.background = background;
        t.foreground = g(1.0);
        t.foreground_subtle = g(0.54);
        t.card = g(0.21);
        t.card_foreground = g(0.97);
        t.popover = g(0.21);
        t.popover_foreground = g(0.97);
        t.primary = g(0.97);
        t.primary_foreground = g(0.16);
        t.secondary = g(0.26);
        t.secondary_foreground = g(0.97);
        t.muted = g(0.26);
        t.muted_foreground = Oklch::new(0.784, 0.004, 121.6).to_color();
        t.accent = g(0.26);
        t.accent_foreground = g(0.97);
        t.destructive = RED_300.to_color();
        t.destructive_foreground = RED_400.to_color();
        t.info_foreground = BLUE_400.to_color();
        t.success_foreground = EMERALD_400.to_color();
        t.warning_foreground = AMBER_400.to_color();
        t.border = g(0.301);
        t.input = g(0.13);
        t.ring = g(0.66);
        t.chart_1 = g(0.9);
        t.chart_2 = g(0.78);
        t.chart_3 = g(0.66);
        t.chart_4 = g(0.54);
        t.chart_5 = g(0.42);
        t.sidebar = g(0.239);
        t.sidebar_foreground = g(0.985);
        t.sidebar_primary = g(1.0);
        t.sidebar_primary_foreground = g(0.178);
        t.sidebar_accent = g(0.178);
        t.sidebar_accent_foreground = g(0.985);
        t.sidebar_border = g(0.301);
        t.sidebar_ring = g(0.66);
        // `--selected-edge` and `--sidebar-element-idle` are declared only in `:root`;
        // they are expressed in terms of the (now dark) background/foreground.
        t.selected_edge = background.with_alpha(0.16);
        t.sidebar_element_idle = t.foreground.with_alpha(0.06);
        t.sidebar_element_hover = ga(1.0, 0.10);
        t.sidebar_count_added = GREEN_300.to_color();
        t.sidebar_count_deleted = RED_300.to_color();
        t.console = g(0.16);
        t.console_foreground = g(0.9);
        t.console_dim = g(0.66);
        t.console_row_hover = ga(1.0, 0.04);
        t.console_row_open = ga(1.0, 0.06);
        t.log_info = g(0.784);
        t.log_debug = g(0.62);
        t.log_warn = Oklch::new(0.828, 0.16, 84.0).to_color();
        t.log_error = Oklch::new(0.74, 0.17, 22.0).to_color();
        t.log_warn_text = Oklch::new(0.9, 0.07, 90.0).to_color();
        t.log_error_text = Oklch::new(0.86, 0.09, 22.0).to_color();
        t.log_number = Oklch::new(0.76, 0.12, 245.0).to_color();
        t.log_bool = Oklch::new(0.78, 0.13, 160.0).to_color();
        t.log_path = Oklch::new(0.78, 0.04, 90.0).to_color();
        t.log_field_error = Oklch::new(0.8, 0.12, 22.0).to_color();
        t.log_underline = ga(1.0, 0.25);
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_background_is_white_and_dark_is_not() {
        assert_eq!(
            Tokens::for_mode(Mode::Light).background.to_rgba8(),
            0xFFFF_FFFF
        );
        assert_ne!(
            Tokens::for_mode(Mode::Dark).background.to_rgba8(),
            0xFFFF_FFFF
        );
    }

    #[test]
    fn dark_overrides_the_foreground() {
        assert_eq!(
            Tokens::for_mode(Mode::Dark).foreground.to_rgba8(),
            0xFFFF_FFFF
        );
    }
}
