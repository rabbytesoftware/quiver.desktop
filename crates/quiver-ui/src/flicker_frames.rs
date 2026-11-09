//! The eight frames of the FlickerSpinner, transcribed from `src/components/ui/flicker-spinner.tsx`.
//!
//! Each frame is 5 rows of 5 dots, most significant bit = leftmost column. `1` = fill-opacity 1,
//! `0` = fill-opacity 0.15. The original SVG strip is 8 frames of 30x30 (dot radius 2, pitch 6).

pub(crate) const FRAMES: [[u8; 5]; 8] = [
    [0b00000, 0b00010, 0b00010, 0b00110, 0b00000],
    [0b00000, 0b00000, 0b00010, 0b01110, 0b00000],
    [0b00000, 0b00000, 0b01000, 0b01110, 0b00000],
    [0b00000, 0b01000, 0b01000, 0b01100, 0b00000],
    [0b00000, 0b01100, 0b01000, 0b01000, 0b00000],
    [0b00000, 0b01110, 0b01000, 0b00000, 0b00000],
    [0b00000, 0b01110, 0b00010, 0b00000, 0b00000],
    [0b00000, 0b00110, 0b00010, 0b00010, 0b00000],
];
