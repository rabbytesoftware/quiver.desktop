//! GPUI components, one file per Coss component, plus the gallery.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod avatar;
mod badge;
mod button;
mod button_style;
mod context_menu;
mod dialog;
mod dot_grid;
mod edge;
pub use edge::{control_edge, inner_highlight, lower_edge, upper_edge};
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
pub mod inspect;
mod label;
mod menu;
mod number_field;
mod overlay;
mod page_header;
mod parts;
mod popup;
mod progress_button;
mod scroll_area;
mod search_input;
mod section_header;
mod select;
mod sheet;
mod slider;
mod spinner;
mod status_glyph;
mod switch;
mod tabs;
mod theme;
mod tile;
mod tile_art;
mod tile_badge;
mod tooltip;
mod wavy_line;

pub use avatar::Avatar;
pub use badge::{Badge, BadgeSize, BadgeVariant};
pub use button::Button;
pub use button_style::{ButtonSize, ButtonVariant};
pub use context_menu::ContextMenu;
pub use dialog::{
    Dialog, DialogDescription, DialogFooter, DialogFooterBare, DialogHeader, DialogPanel,
    DialogTitle,
};
pub use error::{Error, Result};
pub use flicker_spinner::FlickerSpinner;
pub use frame::{Frame, FrameDescription, FrameFooter, FrameHeader, FramePanel, FrameTitle};
pub use gallery::{load_fonts, run_gallery};
pub use icon::{icon, icon_with_stroke, Glyph};
pub use input::{Input, InputEvent, InputState};
pub use label::Label;
pub use menu::{Menu, MenuAlign, MenuEntry, MenuItem};
pub use number_field::NumberField;
pub use page_header::PageHeader;
pub use progress_button::ProgressButton;
pub use scroll_area::ScrollArea;
pub use search_input::SearchInput;
pub use section_header::SectionHeader;
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
pub use tile::{Tile, TileArtKind, TileSize, TileStart};
pub use tile_badge::{TileBadge, TileStatus};
pub use tooltip::{tooltip, TooltipPopup};
pub use wavy_line::WavyLine;
