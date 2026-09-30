use bake::{Error, Result};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Copyright {
    first_year: u16,
    last_year: u16,
    author: String,
}

impl Copyright {
    pub(crate) fn statement(&self) -> String {
        let years = if self.first_year == self.last_year {
            self.first_year.to_string()
        } else {
            format!("{}-{}", self.first_year, self.last_year)
        };

        let punctuation = if self.author.ends_with('.') { "" } else { "." };
        format!("Copyright, {years}, by {}{punctuation}", self.author)
    }

    pub(crate) fn source_comment(&self) -> String {
        format!("// {}", self.statement())
    }
}

pub(crate) fn repository(root: &Path) -> Result<Vec<Copyright>> {
    collect(root, None)
}

pub(crate) fn file(root: &Path, path: &Path) -> Result<Vec<Copyright>> {
    collect(root, Some(path))
}

pub(crate) fn rust_files(root: &Path) -> Result<Vec<String>> {
    let output = git_command(root)
        .args(["ls-files", "--cached", "-z", "--", "*.rs"])
        .output()?;
    ensure_success(&output.status, &output.stderr, "git ls-files")?;

    let mut paths = Vec::new();
    for path in output.stdout.split(|byte| *byte == 0) {
        if path.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(path)
            .map_err(|error| Error::new(format!("tracked Rust path is not UTF-8: {error}")))?;
        if path
            .split('/')
            .any(|part| matches!(part, ".git" | "target" | "vendor"))
        {
            continue;
        }
        paths.push(path.to_owned());
    }

    Ok(paths)
}

fn collect(root: &Path, path: Option<&Path>) -> Result<Vec<Copyright>> {
    let ignored = ignored_revisions(root)?;
    let mut command = git_command(root);
    command.args(["log", "--format=%H%x00%aN%x00%aI"]);
    if path.is_some() {
        command.arg("--follow");
    }
    command
        .arg("--")
        .arg(path.unwrap_or_else(|| Path::new(".")));

    let output = command.output()?;
    ensure_success(&output.status, &output.stderr, "git log")?;

    let output = String::from_utf8_lossy(&output.stdout);
    let mut years_by_author = BTreeMap::<String, (u16, u16)>::new();

    for record in output.lines().filter(|line| !line.is_empty()) {
        let mut fields = record.split('\0');
        let Some(revision) = fields.next() else {
            continue;
        };
        let Some(author) = fields.next() else {
            continue;
        };
        let Some(date) = fields.next() else {
            continue;
        };

        if ignored
            .iter()
            .any(|ignored_revision| revision.starts_with(ignored_revision))
            || author.ends_with("[bot]")
            || author.is_empty()
        {
            continue;
        }

        let Some(year) = date.get(..4).and_then(|year| year.parse::<u16>().ok()) else {
            continue;
        };

        years_by_author
            .entry(author.to_owned())
            .and_modify(|(first_year, last_year)| {
                *first_year = (*first_year).min(year);
                *last_year = (*last_year).max(year);
            })
            .or_insert((year, year));
    }

    let mut copyrights: Vec<_> = years_by_author
        .into_iter()
        .map(|(author, (first_year, last_year))| Copyright {
            first_year,
            last_year,
            author,
        })
        .collect();
    copyrights.sort();
    Ok(copyrights)
}

fn ignored_revisions(root: &Path) -> Result<HashSet<String>> {
    let path = root.join(".git-blame-ignore-revs");
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HashSet::new());
        }
        Err(error) => return Err(Error::from(error)),
    };

    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    command
}

fn ensure_success(status: &std::process::ExitStatus, stderr: &[u8], operation: &str) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    Err(Error::new(format!(
        "{operation} failed: {}",
        String::from_utf8_lossy(stderr).trim()
    )))
}
