// SPDX-License-Identifier: AGPL-3.0-or-later
//! A thin, structured wrapper around the system `git` binary.
//!
//! `anarchie` deliberately uses git as its versioning substrate rather than a
//! bespoke object store (see `specs/versioning-and-git.md`). Shelling out to the
//! installed `git` keeps the binary dependency-light and the on-disk repository
//! a completely ordinary git repository that any developer can inspect.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::store::error::{Result, StoreError};

/// A handle to one git repository (one EHR).
#[derive(Clone, Debug)]
pub struct Git {
    repo: PathBuf,
}

impl Git {
    /// Wrap an existing repository working directory.
    pub fn open(repo: impl Into<PathBuf>) -> Self {
        Self { repo: repo.into() }
    }

    /// The repository working-tree root.
    pub fn root(&self) -> &Path {
        &self.repo
    }

    /// `git init` a new repository at `repo`, configuring a deterministic
    /// default branch so the layout is stable across git versions.
    pub fn init(&self) -> Result<()> {
        self.run(&["init", "--quiet", "--initial-branch=main"])?;
        Ok(())
    }

    /// Stage a path (relative to the repo root).
    pub fn add(&self, rel: &str) -> Result<()> {
        self.run(&["add", "--", rel])?;
        Ok(())
    }

    /// Create a commit. `trailers` are appended as `Key: value` lines so the
    /// openEHR contribution metadata round-trips through `git log`.
    ///
    /// Author identity is forced explicitly so commits are reproducible and do
    /// not depend on ambient `user.name`/`user.email` config.
    pub fn commit(
        &self,
        message: &str,
        author_name: &str,
        author_email: &str,
        committed_at: &str,
        trailers: &[(&str, &str)],
    ) -> Result<String> {
        let mut full = String::from(message);
        if !trailers.is_empty() {
            full.push_str("\n\n");
            for (k, v) in trailers {
                full.push_str(k);
                full.push_str(": ");
                full.push_str(v);
                full.push('\n');
            }
        }

        let author = format!("{author_name} <{author_email}>");
        self.run_with_env(
            &[
                "-c",
                "core.commentchar=;",
                "commit",
                "--quiet",
                "--author",
                &author,
                "--date",
                committed_at,
                "--message",
                &full,
            ],
            &[
                ("GIT_AUTHOR_NAME", author_name),
                ("GIT_AUTHOR_EMAIL", author_email),
                ("GIT_AUTHOR_DATE", committed_at),
                ("GIT_COMMITTER_NAME", author_name),
                ("GIT_COMMITTER_EMAIL", author_email),
                ("GIT_COMMITTER_DATE", committed_at),
            ],
        )?;
        self.head_sha()
    }

    /// The full SHA of `HEAD`.
    pub fn head_sha(&self) -> Result<String> {
        Ok(self.run(&["rev-parse", "HEAD"])?.trim().to_string())
    }

    /// Whether the repository has at least one commit.
    pub fn has_commits(&self) -> bool {
        self.run(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_ok()
    }

    /// Number of commits that have touched `rel`.
    pub fn commit_count(&self, rel: &str) -> Result<u32> {
        if !self.has_commits() {
            return Ok(0);
        }
        let out = self.run(&["rev-list", "--count", "HEAD", "--", rel])?;
        Ok(out.trim().parse().unwrap_or(0))
    }

    /// `git show <revspec>` for an arbitrary blob, e.g. `HEAD:path` or
    /// `<sha>:path`.
    pub fn show(&self, revspec: &str) -> Result<String> {
        self.run(&["show", revspec])
    }

    /// `git log` for one path, formatted as `<sha>\t<iso-date>\t<subject>`.
    pub fn log_path(&self, rel: &str) -> Result<String> {
        self.run(&["log", "--pretty=format:%H%x09%cI%x09%s", "--", rel])
    }

    /// `git diff <from> <to> -- <rel>` (either side may be a commit-ish).
    pub fn diff(&self, from: &str, to: &str, rel: &str) -> Result<String> {
        self.run(&["diff", from, to, "--", rel])
    }

    fn run(&self, args: &[&str]) -> Result<String> {
        self.run_with_env(args, &[])
    }

    fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> Result<String> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&self.repo).args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let output = cmd.output().map_err(StoreError::GitSpawn)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(StoreError::Git {
                command: format!("git {}", args.join(" ")),
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            })
        }
    }
}
