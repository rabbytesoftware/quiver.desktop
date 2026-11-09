//! GPUI components, one file per Coss component, plus the gallery.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod avatar;
mod badge;
mod button;
mod button_style;
mod dialog;
mod dot_grid;
mod elevation;
mod error;
mod flicker_frames;
mod flicker_spinner;
mod focus;
mod frame;
mod gallery;
mod icon;
mod ids;
mod input;
mod label;
mod menu;
mod number_field;
mod overlay;
mod parts;
mod popup;
mod progress_button;
mod scroll_area;
mod select;
mod sheet;
mod slider;
mod spinner;
mod status_glyph;
mod switch;
mod tabs;
mod theme;
mod tooltip;
mod wavy_line;

pub use avatar::Avatar;
pub use badge::{Badge, BadgeSize, BadgeVariant};
pub use button::Button;
pub use button_style::{ButtonSize, ButtonVariant};
pub use dialog::{
    Dialog, DialogDescription, DialogFooter, DialogFooterBare, DialogHeader, DialogPanel,
    DialogTitle,
};
pub use error::{Error, Result};
pub use flicker_spinner::FlickerSpinner;
pub use frame::{Frame, FrameDescription, FrameFooter, FrameHeader, FramePanel, FrameTitle};
pub use gallery::run_gallery;
pub use icon::{icon, Glyph};
pub use input::{Input, InputEvent, InputState};
pub use label::Label;
pub use menu::{Menu, MenuAlign, MenuEntry, MenuItem};
pub use number_field::NumberField;
pub use progress_button::ProgressButton;
pub use scroll_area::ScrollArea;
pub use select::{Select, SelectOption, SelectSize};
pub use sheet::{
    Sheet, SheetDescription, SheetFooter, SheetHeader, SheetPanel, SheetSide, SheetTitle,
    SheetVariant,
};
pub use slider::Slider;
pub use spinner::Spinner;
pub use status_glyph::{GlyphKind, StatusGlyph};
pub use switch::{Switch, SwitchSize};
pub use tabs::{Orientation, TabItem, Tabs, TabsVariant};
pub use theme::{install, window_root, ActiveQuiver, Palette, QuiverTheme};
pub use tooltip::{tooltip, TooltipPopup};
pub use wavy_line::WavyLine;
