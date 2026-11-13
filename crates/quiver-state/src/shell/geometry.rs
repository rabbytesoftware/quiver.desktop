//! Where the window controls sit relative to the rail.

pub const ROW_H: u32 = 34;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

impl SidebarSide {
    pub fn as_stored(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlsEdge {
    Left,
    Right,
}

/// Space the OS paints window controls into, which our chrome must leave empty.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowControls {
    pub edge: ControlsEdge,
    pub width: u32,
}

/// macOS paints its traffic lights over a 64 px spacer on the left; other platforms draw their own.
pub fn window_controls(is_macos: bool) -> Option<WindowControls> {
    is_macos.then_some(WindowControls {
        edge: ControlsEdge::Left,
        width: 64,
    })
}

/// A rail owns the reserve only when it sits on the same edge as the controls; otherwise the chrome row does.
pub fn rail_owns_controls(
    is_macos: bool,
    side: SidebarSide,
) -> bool {
    window_controls(is_macos).is_some_and(|c| {
        matches!(
            (c.edge, side),
            (ControlsEdge::Left, SidebarSide::Left) | (ControlsEdge::Right, SidebarSide::Right)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_height_is_34() {
        assert_eq!(ROW_H, 34);
    }

    #[test]
    fn macos_reserves_64px_on_the_left_where_the_os_paints_the_lights() {
        assert_eq!(
            window_controls(true),
            Some(WindowControls {
                edge: ControlsEdge::Left,
                width: 64
            })
        );
        assert_eq!(window_controls(false), None);
    }

    #[test]
    fn a_left_rail_owns_the_reserve_on_macos_a_right_rail_does_not() {
        assert!(rail_owns_controls(true, SidebarSide::Left));
        assert!(!rail_owns_controls(true, SidebarSide::Right));
        assert!(!rail_owns_controls(false, SidebarSide::Left));
    }
}
