use std::io::{self, IsTerminal};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

pub struct SweepProgress {
    _multi: Option<MultiProgress>,
    bar: Option<ProgressBar>,
}

impl SweepProgress {
    pub fn new(total: usize, message: &str) -> Self {
        // Keep interactive progress on stderr so stdout stays stable for scripts.
        if total == 0 || !io::stderr().is_terminal() {
            return Self {
                _multi: None,
                bar: None,
            };
        }

        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(20));
        let bar = multi.add(ProgressBar::new(total as u64));
        bar.set_style(
            ProgressStyle::with_template("{msg:<13} [{wide_bar:.cyan/blue}] {pos:>4}/{len:<4}")
                .expect("valid progress bar template")
                .progress_chars("=>-"),
        );
        bar.set_message(message.to_owned());

        Self {
            _multi: Some(multi),
            bar: Some(bar),
        }
    }

    pub fn inc(&self, delta: u64) {
        if let Some(bar) = &self.bar {
            bar.inc(delta);
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.bar.is_some()
    }

    pub fn println(&self, line: impl AsRef<str>) {
        if let Some(bar) = &self.bar {
            bar.println(line.as_ref().to_owned());
        }
    }

    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}
