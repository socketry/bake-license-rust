// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::authorship::{self, Copyright};
use crate::source;
use bake::{Error, Result};
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
    let mut output = String::with_capacity(document.len());
    let mut skipped_heading_level = None;
    let mut fence: Option<(char, usize)> = None;

    for line in document.split_inclusive('\n') {
        let clean = line.trim_end_matches('\n').trim_end_matches('\r');

        if let Some((marker, length)) = fence {
            if closes_fence(clean, marker, length) {
                fence = None;
            }
            if skipped_heading_level.is_none() {
                output.push_str(line);
            }
            continue;
        }

        if let Some((marker, length)) = opens_fence(clean) {
            fence = Some((marker, length));
            if skipped_heading_level.is_none() {
                output.push_str(line);
            }
            continue;
        }

        if let Some(level) = skipped_heading_level {
            if let Some((next_level, _)) = heading(clean)
                && next_level <= level
            {
                skipped_heading_level = None;
            } else {
                continue;
            }
        }

        if let Some((level, title)) = heading(clean)
            && title.eq_ignore_ascii_case("license")
        {
            skipped_heading_level = Some(level);
            continue;
        }

        output.push_str(line);
    }

    output
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level)
        || !trimmed
            .as_bytes()
            .get(level)
            .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }

    let title = trimmed[level..].trim().trim_end_matches('#').trim();
    Some((level, title))
}

fn opens_fence(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn closes_fence(line: &str, marker: char, length: usize) -> bool {
    let trimmed = line.trim_start();
    let marker_length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    marker_length >= length && trimmed[marker_length..].trim().is_empty()
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
