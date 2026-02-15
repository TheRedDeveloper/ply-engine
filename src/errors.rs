#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorType {
    /// Thrown if the text measurement function is never provided to clay and you try using
    /// `Clay::text`
    TextMeasurementFunctionNotProvided,
    ArenaCapacityExceeded,
    ElementsCapacityExceeded,
    TextMeasurementCapacityExceeded,
    /// Thrown if you are trying to use an id that's already used by some other element
    DuplicateId,
    /// Floating container require a parent, the following error is thrown if the parent is not
    /// found
    FloatingContainerParentNotFound,
    InternalError,
}

#[derive(Debug, Clone, Copy)]
pub struct Error<'a> {
    pub type_: ErrorType,
    pub text: &'a str,
}
