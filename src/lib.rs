//! License and copyright maintenance tasks for Bake.

mod authorship;
mod source;
mod update;

pub use update::{Update, update};

/// License and copyright maintenance tasks.
pub mod license {
    use bake::{Context, Result};

    /// Refresh `license.md`, the README License section, and Rust source headers.
    #[bake::task]
    pub fn update(context: &mut Context) -> Result<String> {
        let summary = super::update(context.root())?;
        Ok(format!(
            "Changed {} file(s); refreshed {} tracked Rust source file(s)",
            summary.files_changed, summary.source_files,
        ))
    }
}
