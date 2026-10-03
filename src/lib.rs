// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! License and copyright maintenance tasks for Bake.
mod authorship;
mod source;
mod update;

#[cfg(test)]
mod test_support;

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

    #[cfg(test)]
    mod tests {
        use crate::test_support::GitRepository;
        use bake::Registry;

        #[test]
        fn reports_updated_license_and_source_file_counts() {
            let repository = GitRepository::new();
            repository.write("src/lib.rs", "pub fn example() {}\n");
            repository.commit(
                "Alice",
                "alice@example.com",
                "2024-01-02T03:04:05+00:00",
                "Add source",
            );

            let mut context = Registry::new().context(repository.root());
            let result = super::update(&mut context).unwrap();

            assert_eq!(
                result,
                "Changed 2 file(s); refreshed 1 tracked Rust source file(s)"
            );
        }
    }
}
