//! Core traits for versioned API migrations.

/// Converts one API version type into another.
///
/// Service macros use this trait for opt-in compatibility paths where a legacy
/// request is upgraded into the canonical request type, and the canonical
/// response is downgraded back into the legacy response type.
pub trait VersionMigration<From, To> {
    /// Error returned when a version migration cannot be performed.
    type Error: std::fmt::Display + Send + Sync + 'static;

    /// Convert `value` from one API version type into another.
    fn migrate(value: From) -> Result<To, Self::Error>;
}
