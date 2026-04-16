use std::io::{self, IsTerminal};

use indicatif::{ProgressBar, ProgressStyle};

pub struct SweepProgress {
    bar: Option<ProgressBar>,
}

impl SweepProgress {
    pub fn new(total: usize, message: &str) -> Self {
        // Keep interactive progress on stderr so stdout stays stable for scripts.
        if total == 0 || !io::stderr().is_terminal() {
            return Self { bar: None };
        }

        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::with_template("{msg:<13} [{bar:40.cyan/blue}] {pos:>4}/{len:<4}")
                .expect("valid progress bar template")
                .progress_chars("=>-"),
        );
        bar.set_message(message.to_owned());

        Self { bar: Some(bar) }
    }

    pub fn inc(&self, delta: u64) {
        if let Some(bar) = &self.bar {
            bar.inc(delta);
        }
    }

    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}
