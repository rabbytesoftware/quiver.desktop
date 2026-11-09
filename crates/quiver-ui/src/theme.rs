//! `quiver-theme` tokens as GPUI colours, plus the installation into gpui-component's own theme.
//!
//! Components never hard-code a colour: they read [`Palette`] through `cx.quiver()`.

use gpui::{App, AppContext, Global, Hsla, Rgba};
use gpui_component::theme::{Theme, ThemeMode};
use quiver_theme::{Color, FontFamily, Mode, Radius, Tokens};

fn hsla(c: Color) -> Hsla {
    Rgba {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
    .into()
}

macro_rules! palette {
    ($($f:ident),* $(,)?) => {
        /// The semantic colours of the active mode, resolved to GPUI colours.
        #[derive(Debug, Clone, Copy)]
        pub struct Palette { $(pub $f: Hsla,)* }

        impl Palette {
            fn from_tokens(t: &Tokens) -> Self { Self { $($f: hsla(t.$f),)* } }
        }
    };
}

palette!(
    background,
    foreground,
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
    selected_edge,
    foreground_subtle,
    sidebar,
    sidebar_foreground,
    sidebar_accent,
    sidebar_accent_foreground,
    sidebar_border,
);

/// The global theme: the mode and its resolved palette.
#[derive(Debug, Clone, Copy)]
pub struct QuiverTheme {
    pub mode: Mode,
    pub palette: Palette,
}

impl Global for QuiverTheme {}

impl QuiverTheme {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            palette: Palette::from_tokens(&Tokens::for_mode(mode)),
        }
    }

    pub fn is_dark(&self) -> bool {
        self.mode == Mode::Dark
    }
}

/// `cx.quiver()` returns the active [`Palette`].
pub trait ActiveQuiver {
    fn quiver(&self) -> &QuiverTheme;
}

impl ActiveQuiver for App {
    fn quiver(&self) -> &QuiverTheme {
        self.global::<QuiverTheme>()
    }
}

/// Initialises gpui-component once and installs `mode`. Safe to call again to switch mode.
pub fn install(cx: &mut App, mode: Mode) {
    if !cx.has_global::<Theme>() {
        gpui_component::init(cx);
    }
    let q = QuiverTheme::new(mode);
    cx.set_global(q);
    let p = q.palette;
    let theme = Theme::global_mut(cx);
    theme.mode = if q.is_dark() {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    theme.font_family = FontFamily::Sans.family().into();
    theme.mono_font_family = FontFamily::Mono.family().into();
    theme.radius = gpui::px(Radius::MD);
    theme.radius_lg = gpui::px(Radius::LG);
    let c = &mut theme.colors;
    c.background = p.background;
    c.foreground = p.foreground;
    c.border = p.border;
    c.input = p.input;
    c.ring = p.ring;
    c.caret = p.foreground;
    c.selection = p.ring.opacity(0.3);
    c.muted = p.muted;
    c.muted_foreground = p.muted_foreground;
    c.accent = p.accent;
    c.accent_foreground = p.accent_foreground;
    c.popover = p.popover;
    c.popover_foreground = p.popover_foreground;
    c.primary = p.primary;
    c.primary_foreground = p.primary_foreground;
    c.secondary = p.secondary;
    c.secondary_foreground = p.secondary_foreground;
    c.danger = p.destructive;
    c.danger_foreground = p.destructive_foreground;
    c.success = p.success;
    c.success_foreground = p.success_foreground;
    c.warning = p.warning;
    c.warning_foreground = p.warning_foreground;
    c.info = p.info;
    c.info_foreground = p.info_foreground;
    c.switch = p.input;
    c.switch_thumb = p.background;
    c.slider_bar = p.primary;
    c.slider_thumb = p.primary;
    c.scrollbar = Hsla::transparent_black();
    c.scrollbar_thumb = p.foreground.opacity(0.2);
    c.scrollbar_thumb_hover = p.foreground.opacity(0.3);
}

/// Wraps a window's content view in gpui-component's `Root`, which its text inputs require
/// (IME bounds, focus handling). Every window of the app must be opened through this.
pub fn window_root(
    view: impl Into<gpui::AnyView>,
    window: &mut gpui::Window,
    cx: &mut App,
) -> gpui::Entity<gpui_component::Root> {
    cx.new(|cx| gpui_component::Root::new(view, window, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_follows_the_mode() {
        let light = QuiverTheme::new(Mode::Light).palette.background;
        let dark = QuiverTheme::new(Mode::Dark).palette.background;
        assert!(light.l > 0.99);
        assert!(dark.l < 0.3);
    }
}
