use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use serde::{Serialize, Serializer};
use colored::*;

use crate::errors::{FileOrganizerError, SkipReason};

/// Final outcome for each processed file
#[derive(Debug)]
pub enum FileOutcome {
    /// File successfully moved to its destination
    Moved(FileReport),

    /// File was renamed during the move (e.g., conflict resolution)
    Renamed { report: FileReport, new_path: PathBuf },

    /// File was skipped with a reason
    Skipped { src: PathBuf, reason: SkipReason, size: u64 },

    /// File failed due to an error
    Err(FileErrorReport),
}

#[derive(Debug)]
pub struct FileReport {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub action: MoveAction,
    pub size: u64,
}

#[derive(Debug)]
pub enum MoveAction {
    Moved,
    Skipped(SkipReason),
    Renamed(PathBuf),
}

#[derive(Debug)]
pub struct FileErrorReport {
    pub path: PathBuf,
    pub stage: Stage,
    pub error: FileOrganizerError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Scan,
    Classify,
    Move,
    Index,
}

impl Stage {
    pub const VARIANTS: [Stage; 4] = [
        Stage::Scan, 
        Stage::Classify, 
        Stage::Move,
        Stage::Index,
    ];

    #[inline]
    pub fn as_index(&self) -> usize {
        match self {
            Stage::Scan => 0,
            Stage::Classify => 1,
            Stage::Move => 2,
            Stage::Index => 3
        }
    }
}


#[derive(Debug, Clone, Default, Serialize)]
pub struct StageTiming {
    #[serde(serialize_with = "serialize_duration")]
    pub duration: Duration,
    pub files: usize,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct SkipStats {
    counts: [u64; SkipReason::VARIANTS.len()],
    bytes: [u64; SkipReason::VARIANTS.len()],
}

impl SkipStats {
    pub fn record(&mut self, reason: SkipReason, size: u64) {
        let idx = reason.as_index();
        self.counts[idx] += 1;
        self.bytes[idx] += size;
    }

    pub fn count(&self, reason: SkipReason) -> u64 {
        self.counts[reason.as_index()]
    }

    pub fn bytes(&self, reason: SkipReason) -> u64 {
        self.bytes[reason.as_index()]
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StageStats {
    timings: [Option<StageTiming>; Stage::VARIANTS.len()],
}

impl StageStats {
    pub fn record(&mut self, stage: Stage, timing: StageTiming) {
        self.timings[stage.as_index()] = Some(timing);
    }

    pub fn get(&self, stage: Stage) -> Option<&StageTiming> {
        self.timings[stage.as_index()].as_ref()
    }
}


#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub discovered: usize,
    pub processed: usize, 
    pub moved: usize,
    pub renamed: usize,
    pub errors: usize,
    pub bytes_moved: u64,
    pub bytes_renamed: u64,
    pub bytes_skipped: u64,

    pub skip_counts: [usize; SkipReason::VARIANTS.len()],
    pub skip_bytes: [u64; SkipReason::VARIANTS.len()],
    
    #[serde(serialize_with = "serialize_duration")]
    pub duration: Duration,

    pub timings: [Option<StageTiming>; Stage::VARIANTS.len()],
}

impl Summary {
    pub fn from_outcomes(discovered: usize, outcomes: &[FileOutcome], start: Instant) -> Self {
        let mut summary = Summary::default();
        summary.discovered = discovered;

        for outcome in outcomes {
            match outcome {
                FileOutcome::Moved(report) => {
                    summary.moved += 1;
                    summary.processed += 1;
                    summary.bytes_moved += report.size;
                }
                FileOutcome::Renamed { report, .. } => {
                    summary.renamed += 1;
                    summary.processed += 1;
                    summary.bytes_renamed += report.size;
                }
                FileOutcome::Skipped { reason, size, .. } => {
                    let idx = reason.as_index();
                    summary.skip_counts[idx] += 1;
                    summary.skip_bytes[idx] += *size;
                    summary.bytes_skipped += *size;
                    summary.processed += 1;
                }
                FileOutcome::Err(_) => {
                    summary.errors += 1;
                    summary.processed += 1;
                }
            }
        }

        summary.duration = start.elapsed();
        summary
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

impl std::fmt::Display for FileOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOutcome::Moved(report) => write!(
                f,
                "{} {} → {}",
                "✔ Moved".green().bold(),
                report.src.display(),
                report.dest.display()
            ),
            FileOutcome::Renamed { report, new_path } => write!(
                f,
                "{} {} → {}",
                "✎ Renamed".cyan().bold(),
                report.src.display(),
                new_path.display()
            ),
            FileOutcome::Skipped { src, reason, .. } => write!(
                f,
                "{} {} ({})",
                "⚠ Skipped".yellow().bold(),
                src.display(),
                reason
            ),
            FileOutcome::Err(err) => write!(
                f,
                "{} at {:?} stage for {}: {}",
                "✖ Error".red().bold(),
                err.stage,
                err.path.display(),
                err.error
            ),
        }
    }
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", "Summary".bold().blue())?;

        let pct = if self.discovered > 0 {
            (self.processed as f64 / self.discovered as f64) * 100.0
        } else { 0.0 };

        // Overview
        writeln!(f, "  Discovered:  {} entries", self.discovered.to_string().yellow())?;
        writeln!(f, "  Processed:   {} files ({:.1}%)", self.processed.to_string().green(), pct)?;
        writeln!(f, "  Moved:       {} files, {}", self.moved.to_string().green(), format_size(self.bytes_moved))?;
        writeln!(f, "  Renamed:     {} files, {}", self.renamed.to_string().cyan(), format_size(self.bytes_renamed))?;
        writeln!(f, "  Errors:      {} files", self.errors.to_string().red())?;


        // Skips
        if self.skip_counts.iter().any(|&c| c > 0) {
            writeln!(f, "\n{}", "Skips:".bold().blue())?;
            for (i, &count) in self.skip_counts.iter().enumerate() {
                if count > 0 {
                    let reason = SkipReason::VARIANTS[i];
                    let bytes = self.skip_bytes[i];
                    writeln!(f, "  - {:<14} {:<4} files ({})",
                        format!("{:?}", reason),
                        count.to_string().yellow(),
                        format_size(bytes)
                    )?;
                }
            }
        }

        // Stage timings
        writeln!(f, "\n{}", "Stage timings:".bold().blue())?;
        for (i, opt) in self.timings.iter().enumerate() {
            if let Some(timing) = opt {
                let stage = Stage::VARIANTS[i];
                let avg = if timing.files > 0 {
                    timing.duration / (timing.files as u32)
                } else {
                    Duration::from_secs(0)
                };
                writeln!(
                    f,
                    "  {:<10} {:<5} files | avg: {:<8} | total: {:<8}",
                    format!("{:?}", stage).cyan(),
                    timing.files.to_string().green(),
                    format_duration(avg).magenta(),
                    format_duration(timing.duration).magenta(),
                )?;
            }
        }

        Ok(())
    }
}

// --- Helpers ---
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.2} ms", secs * 1000.0)
    } else {
        format!("{:.2} s", secs)
    }
}

fn serialize_duration<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f64(d.as_secs_f64())
}

#[macro_export]
macro_rules! timed_stage {
    (async $summary:expr, $stage:expr, { $($work:tt)* }) => {{
        let start = std::time::Instant::now();
        let result = { $($work)* }.await;
        let elapsed = start.elapsed();

        let idx = $stage.as_index();
        let entry = $summary.timings[idx].get_or_insert_with(Default::default);
        entry.duration += elapsed;
        entry.files += 1;

        result
    }};
}
