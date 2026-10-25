//! The `compas-sdtool` sidecar: the player's own library code as a command-line
//! program, used to build the music database and album-art cache.

mod events;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::card::{Card, Model};
use crate::error::{Error, Result};
use crate::progress::{OnProgress, Progress};

pub use events::{ArtDone, LibraryInfo, LibraryUpdate, ScanDone};

/// Last stderr lines kept to explain a crash.
const STDERR_TAIL: usize = 5;

#[derive(Debug, Clone)]
pub struct SdTool {
    exe: PathBuf,
}

/// Options shared by `scan` and `art`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanOptions {
    /// Re-read every file, as the player's "refresh all metadata" does.
    pub full: bool,
    /// Extra seconds added to the card's file times.
    pub mtime_offset: Option<i64>,
}

impl SdTool {
    pub fn new(exe: impl Into<PathBuf>) -> Self {
        Self { exe: exe.into() }
    }

    pub async fn info(&self, card: &Card) -> Result<LibraryInfo> {
        self.run(args("info", card, Vec::new()), &|_| {}).await
    }

    pub async fn scan(
        &self,
        card: &Card,
        options: ScanOptions,
        on_progress: OnProgress<'_>,
    ) -> Result<ScanDone> {
        let mut extra = Vec::new();
        if options.full {
            extra.push("--full".into());
        }
        extra.extend(offset_args(options.mtime_offset));
        self.run(args("scan", card, extra), on_progress).await
    }

    pub async fn art(
        &self,
        card: &Card,
        board: Option<Model>,
        mtime_offset: Option<i64>,
        on_progress: OnProgress<'_>,
    ) -> Result<ArtDone> {
        let mut extra: Vec<OsString> = Vec::new();
        if let Some(model) = board {
            extra.extend(["--board".into(), model.board().into()]);
        }
        extra.extend(offset_args(mtime_offset));
        self.run(args("art", card, extra), on_progress).await
    }

    /// What a card needs after its music changed: scan the library, then build
    /// the album art, as one run.
    pub async fn update_library(
        &self,
        card: &Card,
        on_progress: OnProgress<'_>,
    ) -> Result<LibraryUpdate> {
        let scan = self.scan(card, ScanOptions::default(), on_progress).await?;
        // The library is usable without covers, so a failed art step is a warning.
        let (art, art_error) = match self.art(card, None, None, on_progress).await {
            Ok(art) => (Some(art), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Ok(LibraryUpdate {
            scan,
            art,
            art_error,
        })
    }

    /// Runs the tool, forwarding progress lines, and returns its final object.
    async fn run<T: DeserializeOwned>(
        &self,
        args: Vec<OsString>,
        on_progress: OnProgress<'_>,
    ) -> Result<T> {
        let mut command = Command::new(&self.exe);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000); // no console window

        let mut child = crate::process::spawn(&mut command)
            .await
            .map_err(|e| Error::Sdtool {
                code: "missing_tool".into(),
                message: format!("{}: {e}", self.exe.display()),
            })?;
        let stderr = child.stderr.take().map(BufReader::new);
        let tail = tokio::spawn(async move {
            let mut lines = Vec::new();
            if let Some(mut reader) = stderr {
                let mut line = String::new();
                while reader.read_line(&mut line).await.is_ok_and(|n| n > 0) {
                    lines.push(line.trim_end().to_string());
                    if lines.len() > STDERR_TAIL {
                        lines.remove(0);
                    }
                    line.clear();
                }
            }
            lines.join("\n")
        });

        let mut final_object = None;
        let mut failure = None;
        let mut lines = BufReader::new(child.stdout.take().expect("stdout is piped")).lines();
        while let Some(line) = lines.next_line().await? {
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            match value["event"].as_str() {
                Some("progress") => forward_progress(&value, on_progress),
                Some("error") => failure = Some(tool_error(&value)),
                _ => final_object = Some(value),
            }
        }
        let status = child.wait().await?;
        let stderr_tail = tail.await.unwrap_or_default();

        if let Some(error) = failure {
            // `art_failed` means the rest of the covers were written: keep the counts.
            return match (&error, final_object) {
                (Error::Sdtool { code, message }, Some(mut done)) if code == "art_failed" => {
                    done["warning"] = friendly_message(code, message).into();
                    serde_json::from_value(done).map_err(|_| error)
                }
                _ => Err(error),
            };
        }
        match final_object.map(serde_json::from_value) {
            Some(Ok(done)) if status.success() => Ok(done),
            _ => Err(Error::Sdtool {
                code: "crashed".into(),
                message: format!("exit status {status}; {stderr_tail}"),
            }),
        }
    }
}

fn args(command: &str, card: &Card, extra: Vec<OsString>) -> Vec<OsString> {
    let mut all: Vec<OsString> = vec![command.into(), card.root().into()];
    all.extend(extra);
    all
}

fn offset_args(offset: Option<i64>) -> Vec<OsString> {
    offset.map_or_else(Vec::new, |s| {
        vec!["--mtime-offset".into(), s.to_string().into()]
    })
}

fn forward_progress(value: &Value, on_progress: OnProgress<'_>) {
    let phase = value["phase"].as_str().and_then(events::phase);
    if let (Some(phase), Some(done), Some(total)) =
        (phase, value["done"].as_u64(), value["total"].as_u64())
    {
        on_progress(Progress { phase, done, total });
    }
}

fn tool_error(value: &Value) -> Error {
    Error::Sdtool {
        code: value["code"].as_str().unwrap_or("unknown").to_string(),
        message: value["message"].as_str().unwrap_or_default().to_string(),
    }
}

/// What the user reads for a tool error code; unknown codes show the tool's text.
pub fn friendly_message(code: &str, message: &str) -> String {
    let text = match code {
        "missing_tool" => "The library tool (compas-sdtool) could not be started.",
        "crashed" => {
            "The library tool stopped unexpectedly. The card was not changed by this step."
        }
        "usage" => "The library tool was called with the wrong arguments.",
        "card_not_found" => "The card folder was not found. Is the card still connected?",
        "database_open_failed" => "The library database on the card could not be opened.",
        "walk_failed" => "The folders on the card could not be read.",
        "no_music_found" => "No music was found on the card.",
        "worker_failed" => "The tag reader could not run, so the library was left unchanged.",
        "write_failed" => "The library could not be written. Is the card full or write-protected?",
        "bad_board" => "That player model is not known to the library tool.",
        "art_failed" => "Some album covers could not be written; the others were.",
        "out_of_memory" => "The computer ran out of memory.",
        _ => return message.to_string(),
    };
    if message.is_empty() || code == "crashed" || code == "missing_tool" {
        format!("{text} {message}").trim().to_string()
    } else {
        text.to_string()
    }
}

#[cfg(all(test, unix))]
mod tests;
