use serde::{Deserialize, Serialize};

pub const WORD_LENGTH: usize = 5;
pub const MAX_GUESSES: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TileState {
    Empty,
    Filled,
    Correct,   // Green - right letter, right position
    Present,   // Yellow - right letter, wrong position
    Absent,    // Gray - letter not in word
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameStatus {
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Hard,
}

/// Evaluate a guess against the answer.
/// Returns a Vec of TileState for each position.
pub fn evaluate_guess(guess: &str, answer: &str) -> Vec<TileState> {
    let guess: Vec<char> = guess.to_lowercase().chars().collect();
    let answer: Vec<char> = answer.to_lowercase().chars().collect();
    let mut result = vec![TileState::Absent; WORD_LENGTH];
    let mut answer_used = vec![false; WORD_LENGTH];

    // First pass: mark correct positions
    for i in 0..WORD_LENGTH {
        if i < guess.len() && i < answer.len() && guess[i] == answer[i] {
            result[i] = TileState::Correct;
            answer_used[i] = true;
        }
    }

    // Second pass: mark present letters (wrong position)
    for i in 0..WORD_LENGTH {
        if result[i] == TileState::Correct {
            continue;
        }
        if i >= guess.len() {
            continue;
        }
        for j in 0..WORD_LENGTH {
            if j >= answer.len() {
                break;
            }
            if !answer_used[j] && guess[i] == answer[j] {
                result[i] = TileState::Present;
                answer_used[j] = true;
                break;
            }
        }
    }

    result
}

/// Check if the current guess is valid in hard mode.
/// In hard mode: all green letters must be reused in correct positions,
/// all yellow letters must be included somewhere in the guess.
pub fn validate_hard_mode(
    guess: &str,
    previous_guesses: &[(String, Vec<TileState>)],
) -> Option<String> {
    let guess_chars: Vec<char> = guess.to_lowercase().chars().collect();

    for (prev_word, prev_states) in previous_guesses {
        let prev_chars: Vec<char> = prev_word.to_lowercase().chars().collect();

        // Check greens: same position must have same letter
        for (i, state) in prev_states.iter().enumerate() {
            if *state == TileState::Correct {
                if i >= guess_chars.len() || guess_chars[i] != prev_chars[i] {
                    return Some(format!(
                        "Letter {} must be in position {}",
                        prev_chars[i].to_uppercase(),
                        i + 1
                    ));
                }
            }
        }

        // Check yellows: letter must appear somewhere in the guess
        for (i, state) in prev_states.iter().enumerate() {
            if *state == TileState::Present {
                let ch = prev_chars[i];
                if !guess_chars.contains(&ch) {
                    return Some(format!(
                        "Guess must contain letter {}",
                        ch.to_uppercase()
                    ));
                }
            }
        }
    }

    None
}
