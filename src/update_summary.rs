// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// Counts of source files inspected and project files changed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdateSummary {
    pub source_files: usize,
    pub files_changed: usize,
}
