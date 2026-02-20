/// Horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AlignX {
    #[default]
    Left,
    CenterX,
    Right,
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AlignY {
    #[default]
    Top,
    CenterY,
    Bottom,
}
