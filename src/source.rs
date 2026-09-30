// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::authorship::Copyright;

pub(crate) fn update_source(contents: &str, copyrights: &[Copyright]) -> String {
    let lines: Vec<&str> = contents.split_inclusive('\n').collect();
    let mut prefix_length = 0;

    if lines
        .first()
        .is_some_and(|line| line.starts_with("#!") && !line.starts_with("#!["))
    {
        prefix_length += 1;
    }

    while lines
        .get(prefix_length)
        .is_some_and(|line| clean_line(line).starts_with("#!["))
    {
        let mut open_brackets = 0isize;
        let mut in_string = false;
        let mut escaped = false;
        while let Some(line) = lines.get(prefix_length) {
            for character in clean_line(line).chars() {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if character == '\\' {
                        escaped = true;
                    } else if character == '"' {
                        in_string = false;
                    }
                    continue;
                }

                match character {
                    '"' => in_string = true,
                    '[' => open_brackets += 1,
                    ']' => open_brackets -= 1,
                    _ => {}
                }
            }
            prefix_length += 1;
            if open_brackets <= 0 {
                break;
            }
        }
    }

    let mut leading_comments = Vec::new();
    while let Some(line) = lines.get(prefix_length + leading_comments.len()) {
        let clean = clean_line(line);
        if clean.trim().is_empty() || clean.trim_start().starts_with("//") {
            leading_comments.push(*line);
        } else {
            break;
        }
    }

    let mut documentation: Vec<_> = leading_comments
        .into_iter()
        .filter(|line| !is_license_metadata(clean_line(line)))
        .collect();
    while documentation
        .first()
        .is_some_and(|line| clean_line(line).trim().is_empty())
    {
        documentation.remove(0);
    }
    while documentation
        .last()
        .is_some_and(|line| clean_line(line).trim().is_empty())
    {
        documentation.pop();
    }

    let body_start = prefix_length
        + lines[prefix_length..]
            .iter()
            .take_while(|line| {
                let clean = clean_line(line);
                clean.trim().is_empty() || clean.trim_start().starts_with("//")
            })
            .count();

    let mut output = String::new();
    for line in &lines[..prefix_length] {
        output.push_str(line);
    }
    if prefix_length > 0 {
        output.push('\n');
    }

    output.push_str("// Released under the MIT License.\n");
    for copyright in copyrights {
        output.push_str(&copyright.source_comment());
        output.push('\n');
    }
    output.push('\n');

    for line in documentation {
        output.push_str(line);
    }
    if !output.ends_with('\n') && !lines[body_start..].is_empty() {
        output.push('\n');
    }
    for line in &lines[body_start..] {
        output.push_str(line);
    }

    output
}

fn is_license_metadata(line: &str) -> bool {
    let Some(comment) = line.trim_start().strip_prefix("//") else {
        return false;
    };
    let comment = comment.trim_start();
    let comment = comment
        .strip_prefix('!')
        .or_else(|| comment.strip_prefix('/'))
        .unwrap_or(comment)
        .trim_start();
    comment.starts_with("Released under ")
        || comment.starts_with("Copyright,")
        || comment.starts_with("SPDX-License-Identifier:")
}

fn clean_line(line: &str) -> &str {
    line.trim_end_matches('\n').trim_end_matches('\r')
}
