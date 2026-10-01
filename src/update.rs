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
    let Ok(root) = to_mdast(document, &ParseOptions::default()) else {
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
    temporary
        .persist(&target_path)
        .map_err(|error| Error::from(error.error))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::remove_license_section;

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
}
