// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::authorship::{self, Copyright};
use crate::source;
use bake::{Error, Result};
use socketry_markdown::{ParseOptions, mdast::Node, to_mdast};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

const MIT_LICENSE: &str = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Update {
    pub source_files: usize,
    pub files_changed: usize,
}

pub fn update(root: &Path) -> Result<Update> {
    let repository_copyrights = authorship::repository(root)?;
    let mut summary = Update::default();

    let license_path = root.join("license.md");
    let license = license_document(&repository_copyrights);
    if replace_if_changed(&license_path, &license)? {
        summary.files_changed += 1;
    }

    let readme_path = root.join("readme.md");
    if readme_path.is_file() {
        let readme = fs::read_to_string(&readme_path)?;
        let updated = remove_license_section(&readme);
        if replace_if_changed(&readme_path, &updated)? {
            summary.files_changed += 1;
        }
    }

    for relative_path in authorship::rust_files(root)? {
        let path = root.join(&relative_path);
        if !path.is_file() {
            continue;
        }

        let copyrights = authorship::file(root, Path::new(&relative_path))?;
        if copyrights.is_empty() {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .map_err(|error| Error::new(format!("{}: {error}", path.display())))?;
        let updated = source::update_source(&contents, &copyrights);
        if replace_if_changed(&path, &updated)? {
            summary.files_changed += 1;
        }
        summary.source_files += 1;
    }

    Ok(summary)
}

fn license_document(copyrights: &[Copyright]) -> String {
    let mut output = String::from("# MIT License\n\n");
    for copyright in copyrights {
        output.push_str(&copyright.statement());
        output.push_str("  \n");
    }
    if !copyrights.is_empty() {
        output.push('\n');
    }
    output.push_str(MIT_LICENSE);
    output.push('\n');
    output
}

fn remove_license_section(document: &str) -> String {
    remove_parsed_license_section(document, to_mdast(document, &ParseOptions::default()))
}

fn remove_parsed_license_section(
    document: &str,
    parsed: std::result::Result<Node, socketry_markdown::message::Message>,
) -> String {
    let Ok(root) = parsed else {
        return document.to_owned();
    };
    let Some(children) = root.children() else {
        return document.to_owned();
    };

    let mut output = String::with_capacity(document.len());
    let mut copied_until = 0;

    for (index, node) in children.iter().enumerate() {
        let Node::Heading(heading) = node else {
            continue;
        };
        if !heading_text(node).eq_ignore_ascii_case("license") {
            continue;
        }
        let Some(position) = node.position() else {
            continue;
        };
        let start = position.start.offset;
        if start < copied_until {
            continue;
        }

        let end = children
            .iter()
            .skip(index + 1)
            .find_map(|next| {
                let Node::Heading(next_heading) = next else {
                    return None;
                };
                (next_heading.depth <= heading.depth)
                    .then(|| next.position().map(|position| position.start.offset))
                    .flatten()
            })
            .unwrap_or(document.len());

        output.push_str(&document[copied_until..start]);
        copied_until = end;
    }

    output.push_str(&document[copied_until..]);
    output
}

fn heading_text(node: &Node) -> String {
    let text = node.text_content();
    text.strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .or_else(|| text.strip_suffix('\r'))
        .unwrap_or(text.as_str())
        .to_owned()
}

fn replace_if_changed(path: &Path, contents: &str) -> Result<bool> {
    let target_path = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    match fs::read_to_string(&target_path) {
        Ok(existing) if existing == contents => return Ok(false),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::from(error)),
    }

    let parent = target_path
        .parent()
        .ok_or_else(|| Error::new("updated file has no parent directory"))?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents.as_bytes())?;
    if let Ok(metadata) = fs::metadata(&target_path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.as_file().sync_all()?;
    persist(temporary, &target_path)?;
    Ok(true)
}

fn persist(temporary: NamedTempFile, path: &Path) -> Result<()> {
    temporary
        .persist(path)
        .map_err(|error| Error::from(error.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorship::Copyright;
    use crate::test_support::GitRepository;
    use socketry_markdown::mdast::{Heading, Node, Root, Text};
    use socketry_markdown::unist::Position;
    use std::fs;
    use tempfile::tempdir;

    fn message() -> socketry_markdown::message::Message {
        socketry_markdown::message::Message {
            place: None,
            reason: "invalid markdown".to_owned(),
            rule_id: Box::new("test".to_owned()),
            source: Box::new("test".to_owned()),
        }
    }

    fn heading(title: &str, depth: u8, start: Option<usize>) -> Node {
        Node::Heading(Heading {
            children: vec![Node::Text(Text {
                value: title.to_owned(),
                position: None,
            })],
            position: start
                .map(|start| Position::new(1, start + 1, start, 1, start + 2, start + 1)),
            depth,
        })
    }

    fn root(children: Vec<Node>) -> Node {
        Node::Root(Root {
            children,
            position: None,
        })
    }

    #[test]
    fn removes_the_license_section_until_the_next_equal_or_higher_heading() {
        let document = "# Project\n\n## License\n\nMIT terms.\n\n### Details\n\nMore terms.\n\n## Usage\n\nUse the project.\n";
        assert_eq!(
            remove_license_section(document),
            "# Project\n\n## Usage\n\nUse the project.\n"
        );
    }

    #[test]
    fn leaves_license_text_inside_html_blocks_untouched() {
        let document = "# Project\n\n<div>\n## License\n</div>\n\n## Usage\n\nUse the project.\n";
        assert_eq!(remove_license_section(document), document);
    }

    #[test]
    fn leaves_documents_without_a_license_heading_untouched() {
        let document = "# Project\n\n## Usage\n\nUse the project.\n";
        assert_eq!(remove_license_section(document), document);
    }

    #[test]
    fn removes_case_insensitive_license_sections_through_the_end_of_a_document() {
        let document = "# Project\n\n## LICENSE\n\nMIT terms.\n";
        assert_eq!(remove_license_section(document), "# Project\n\n");
    }

    #[test]
    fn ignores_license_headings_nested_inside_a_removed_section() {
        let document =
            "# Project\n\n## License\n\n### License\n\nNested terms.\n\n## Usage\n\nUse it.\n";
        assert_eq!(
            remove_license_section(document),
            "# Project\n\n## Usage\n\nUse it.\n"
        );
    }

    #[test]
    fn preserves_the_document_when_parsing_fails_or_returns_a_non_container() {
        let document = "Keep this document.\n";
        assert_eq!(
            remove_parsed_license_section(document, Err(message())),
            document
        );
        assert_eq!(
            remove_parsed_license_section(
                document,
                Ok(Node::Text(Text {
                    value: document.to_owned(),
                    position: None,
                }))
            ),
            document
        );
    }

    #[test]
    fn leaves_heading_nodes_without_source_positions_untouched() {
        let document = "## License\n\nMIT terms.\n";
        assert_eq!(
            remove_parsed_license_section(document, Ok(root(vec![heading("License", 2, None)]))),
            document
        );
    }

    #[test]
    fn removes_a_license_section_when_the_next_heading_has_no_position() {
        let document = "Before.\n\n## License\n\nMIT terms.\n\n## Usage\n";
        let start = document.find("## License").unwrap();
        assert_eq!(
            remove_parsed_license_section(
                document,
                Ok(root(vec![
                    heading("License", 2, Some(start)),
                    heading("Usage", 2, None),
                ])),
            ),
            "Before.\n\n"
        );
    }

    #[test]
    fn strips_newline_suffixes_from_markdown_heading_text() {
        for (text, expected) in [
            ("License\r\n", "License"),
            ("License\n", "License"),
            ("License\r", "License"),
            ("License", "License"),
        ] {
            assert_eq!(
                heading_text(&Node::Text(Text {
                    value: text.to_owned(),
                    position: None,
                })),
                expected
            );
        }
    }

    #[test]
    fn renders_license_copyrights_with_an_optional_author_section() {
        let empty = license_document(&[]);
        assert!(empty.starts_with("# MIT License\n\nPermission is hereby granted"));

        let with_authors = license_document(&[
            Copyright::new(2020, 2020, "Alice"),
            Copyright::new(2021, 2024, "Bob."),
        ]);
        assert!(with_authors.starts_with(
            "# MIT License\n\nCopyright, 2020, by Alice.  \nCopyright, 2021-2024, by Bob.  \n\nPermission is hereby granted"
        ));
        assert!(with_authors.ends_with("SOFTWARE.\n"));
    }

    #[test]
    fn updates_license_readme_and_tracked_sources_idempotently() {
        let repository = GitRepository::new();
        repository.write(
            "readme.md",
            "# Project\n\n## License\n\nMIT terms.\n\n## Usage\n\nUse the project.\n",
        );
        repository.write(
            "src/lib.rs",
            "//! Module documentation.\npub fn example() {}\n",
        );
        repository.commit(
            "Alice",
            "alice@example.com",
            "2024-01-02T03:04:05+00:00",
            "Add project files",
        );

        let first = update(repository.root()).unwrap();
        assert_eq!(
            first,
            Update {
                source_files: 1,
                files_changed: 3,
            }
        );
        assert!(
            fs::read_to_string(repository.root().join("license.md"))
                .unwrap()
                .starts_with("# MIT License\n\nCopyright, 2024, by Alice.")
        );
        assert_eq!(
            fs::read_to_string(repository.root().join("readme.md")).unwrap(),
            "# Project\n\n## Usage\n\nUse the project.\n"
        );
        assert!(
            fs::read_to_string(repository.root().join("src/lib.rs"))
                .unwrap()
                .contains("// Copyright, 2024, by Alice.\n")
        );

        assert_eq!(
            update(repository.root()).unwrap(),
            Update {
                source_files: 1,
                files_changed: 0,
            }
        );
    }

    #[test]
    fn skips_missing_readmes_deleted_sources_and_files_without_authors() {
        let repository = GitRepository::new();
        repository.write("src/deleted.rs", "pub fn deleted() {}\n");
        repository.commit(
            "Alice",
            "alice@example.com",
            "2024-01-02T03:04:05+00:00",
            "Add deleted source",
        );
        repository.write("src/generated.rs", "pub fn generated() {}\n");
        repository.commit(
            "automation[bot]",
            "bot@example.com",
            "2025-02-03T04:05:06+00:00",
            "Add generated source",
        );
        fs::remove_file(repository.root().join("src/deleted.rs")).unwrap();
        assert!(
            authorship::rust_files(repository.root())
                .unwrap()
                .contains(&"src/deleted.rs".to_owned())
        );

        assert_eq!(
            update(repository.root()).unwrap(),
            Update {
                source_files: 0,
                files_changed: 1,
            }
        );
        assert!(!repository.root().join("readme.md").exists());
        assert_eq!(
            fs::read_to_string(repository.root().join("src/generated.rs")).unwrap(),
            "pub fn generated() {}\n"
        );
    }

    #[test]
    fn reports_invalid_readme_and_source_text() {
        let readme = GitRepository::new();
        readme.write("readme.md", [0xff]);
        readme.write("src/lib.rs", "pub fn example() {}\n");
        readme.commit(
            "Alice",
            "alice@example.com",
            "2024-01-02T03:04:05+00:00",
            "Add invalid readme",
        );
        assert!(update(readme.root()).is_err());

        let source = GitRepository::new();
        source.write("src/lib.rs", [0xff]);
        source.commit(
            "Alice",
            "alice@example.com",
            "2024-01-02T03:04:05+00:00",
            "Add invalid source",
        );
        assert!(update(source.root()).is_err());
    }

    #[test]
    fn replace_if_changed_creates_updates_and_preserves_file_permissions() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("license.md");
        assert!(replace_if_changed(&path, "first\n").unwrap());
        assert!(!replace_if_changed(&path, "first\n").unwrap());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        }

        assert!(replace_if_changed(&path, "second\n").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn reports_invalid_target_text_missing_parent_and_unreadable_directories() {
        let directory = tempdir().unwrap();
        let invalid = directory.path().join("invalid.md");
        fs::write(&invalid, [0xff]).unwrap();
        assert!(replace_if_changed(&invalid, "content").is_err());

        assert!(replace_if_changed(Path::new(""), "content").is_err());
        assert!(replace_if_changed(&directory.path().join("missing/file.md"), "content").is_err());

        let folder = directory.path().join("folder");
        fs::create_dir(&folder).unwrap();
        assert!(replace_if_changed(&folder, "content").is_err());
    }

    #[test]
    fn reports_persist_failures() {
        let directory = tempdir().unwrap();
        let temporary = NamedTempFile::new_in(directory.path()).unwrap();

        assert!(persist(temporary, directory.path()).is_err());
    }
}
