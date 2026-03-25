use serde::{Deserialize, Serialize};
use gloo::storage::{LocalStorage, Storage};

const STATS_KEY: &str = "ohmywordle_stats";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    pub played: u32,
    pub won: u32,
    pub current_streak: u32,
    pub max_streak: u32,
    pub guess_distribution: [u32; 6], // index 0 = solved in 1 guess, etc.
}

impl Stats {
    pub fn load() -> Self {
        LocalStorage::get(STATS_KEY).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = LocalStorage::set(STATS_KEY, self);
    }

    pub fn record_win(&mut self, num_guesses: usize) {
        self.played += 1;
        self.won += 1;
        self.current_streak += 1;
        if self.current_streak > self.max_streak {
            self.max_streak = self.current_streak;
        }
        if num_guesses >= 1 && num_guesses <= 6 {
            self.guess_distribution[num_guesses - 1] += 1;
        }
        self.save();
    }

    pub fn record_loss(&mut self) {
        self.played += 1;
        self.current_streak = 0;
        self.save();
    }

    pub fn win_percentage(&self) -> u32 {
        if self.played == 0 {
            0
        } else {
            (self.won * 100) / self.played
        }
    }
}
