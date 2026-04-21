use yew::prelude::*;
use wasm_bindgen::JsCast;
use gloo::utils::window;
use gloo::storage::{LocalStorage, Storage};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::seq::SliceRandom;

use crate::game::{
    evaluate_guess, validate_hard_mode, Difficulty, GameStatus, TileState, WORD_LENGTH,
    MAX_GUESSES,
};
use crate::stats::Stats;
use crate::words::{is_valid_word, valid_words};

const DIFFICULTY_KEY: &str = "ohmywordle_difficulty";
const INSTALL_BANNER_KEY: &str = "ohmywordle_install_banner_dismissed";

/// Encode a word to a URL-safe base64 string
fn encode_word(word: &str) -> String {
    URL_SAFE_NO_PAD.encode(word.as_bytes())
}

/// Decode a URL-safe base64 string to a word
fn decode_word(encoded: &str) -> Option<String> {
    URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() == WORD_LENGTH && s.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Get the shared word from URL query parameter ?w=<encoded>
fn get_shared_word() -> Option<String> {
    let search = window().location().search().ok()?;
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let encoded = params.get("w")?;
    decode_word(&encoded)
}

/// Detect mobile platform and return install instructions, or None for desktop
fn get_install_instructions() -> Option<(&'static str, &'static str)> {
    if let Ok(ua) = window().navigator().user_agent() {
        let ua_lower = ua.to_lowercase();
        if ua_lower.contains("iphone") || ua_lower.contains("ipad") || ua_lower.contains("ipod") {
            return Some(("📤", "Tap the Share button then select \"Add to Home Screen\""));
        }
        if ua_lower.contains("android") {
            return Some(("⋮", "Tap the browser menu then select \"Add to Home Screen\""));
        }
    }
    None
}

/// Pick a random word from the word list
fn random_word() -> String {
    let words = valid_words();
    let mut rng = rand::thread_rng();
    words.choose(&mut rng).unwrap_or(&"crane").to_string()
}

#[derive(Clone, Debug)]
pub struct GuessRow {
    pub word: String,
    pub states: Vec<TileState>,
}

pub enum Msg {
    KeyPress(String),
    Backspace,
    Enter,
    NewGame,
    ShareUrl,
    ToggleDifficulty,
    ShowStats,
    HideStats,
    ShowHowToPlay,
    HideHowToPlay,
    HideMessage,
    CopyDone,
    ShowCreateLink,
    HideCreateLink,
    CreateLinkInput(String),
    CopyCreateLink,
    DismissInstallBanner,
}

pub struct App {
    answer: String,
    current_guess: String,
    guesses: Vec<GuessRow>,
    status: GameStatus,
    difficulty: Difficulty,
    stats: Stats,
    message: Option<String>,
    show_stats: bool,
    show_how_to_play: bool,
    is_shared_game: bool,
    letter_states: std::collections::HashMap<char, TileState>,
    copy_done: bool,
    show_create_link: bool,
    create_link_word: String,
    show_install_banner: bool,
}

impl App {
    fn letter_state_class(state: &TileState) -> &'static str {
        match state {
            TileState::Correct => "correct",
            TileState::Present => "present",
            TileState::Absent => "absent",
            TileState::Filled => "filled",
            TileState::Empty => "",
        }
    }

    fn update_letter_states(&mut self) {
        for row in &self.guesses {
            for (i, state) in row.states.iter().enumerate() {
                let ch = row.word.chars().nth(i).unwrap_or(' ');
                let existing = self.letter_states.get(&ch);
                let should_update = match existing {
                    None => true,
                    Some(TileState::Correct) => false,
                    Some(TileState::Present) => *state == TileState::Correct,
                    Some(TileState::Absent) => *state != TileState::Absent,
                    _ => true,
                };
                if should_update {
                    self.letter_states.insert(ch, state.clone());
                }
            }
        }
    }

    fn show_message(&mut self, ctx: &Context<Self>, msg: String) {
        self.message = Some(msg);
        let link = ctx.link().clone();
        gloo::timers::callback::Timeout::new(2000, move || {
            link.send_message(Msg::HideMessage);
        })
        .forget();
    }

    fn build_url_for_word(word: &str) -> String {
        let encoded = encode_word(word);
        let location = window().location();
        let origin = location.origin().unwrap_or_default();
        let pathname = location.pathname().unwrap_or_default();
        format!("{}{}?w={}", origin, pathname, encoded)
    }

    fn build_share_url(&self) -> String {
        Self::build_url_for_word(&self.answer)
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let (answer, is_shared_game) = if let Some(word) = get_shared_word() {
            (word, true)
        } else {
            (random_word(), false)
        };

        let difficulty = LocalStorage::get::<Difficulty>(DIFFICULTY_KEY)
            .unwrap_or(Difficulty::Easy);

        let install_banner_dismissed = LocalStorage::get::<bool>(INSTALL_BANNER_KEY)
            .unwrap_or(false);
        let show_install_banner = !install_banner_dismissed && get_install_instructions().is_some();

        App {
            answer,
            current_guess: String::new(),
            guesses: Vec::new(),
            status: GameStatus::Playing,
            difficulty,
            stats: Stats::load(),
            message: None,
            show_stats: false,
            show_how_to_play: false,
            is_shared_game,
            letter_states: std::collections::HashMap::new(),
            copy_done: false,
            show_create_link: false,
            create_link_word: String::new(),
            show_install_banner,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::KeyPress(key) => {
                if self.status != GameStatus::Playing {
                    return false;
                }
                if self.current_guess.len() < WORD_LENGTH {
                    let ch = key.to_lowercase().chars().next().unwrap_or(' ');
                    if ch.is_ascii_alphabetic() {
                        self.current_guess.push(ch);
                        return true;
                    }
                }
                false
            }
            Msg::Backspace => {
                if self.status != GameStatus::Playing {
                    return false;
                }
                self.current_guess.pop();
                true
            }
            Msg::Enter => {
                if self.status != GameStatus::Playing {
                    return false;
                }
                if self.current_guess.len() != WORD_LENGTH {
                    self.show_message(ctx, "Not enough letters".to_string());
                    return true;
                }
                if !is_valid_word(&self.current_guess) {
                    self.show_message(ctx, "Not in word list".to_string());
                    return true;
                }
                // Hard mode validation
                if self.difficulty == Difficulty::Hard {
                    let prev: Vec<(String, Vec<TileState>)> = self
                        .guesses
                        .iter()
                        .map(|r| (r.word.clone(), r.states.clone()))
                        .collect();
                    if let Some(err) = validate_hard_mode(&self.current_guess, &prev) {
                        self.show_message(ctx, err);
                        return true;
                    }
                }

                let states = evaluate_guess(&self.current_guess, &self.answer);
                let guess_word = self.current_guess.clone();
                self.guesses.push(GuessRow {
                    word: guess_word.clone(),
                    states: states.clone(),
                });
                self.update_letter_states();
                self.current_guess.clear();

                let won = states.iter().all(|s| *s == TileState::Correct);
                if won {
                    self.status = GameStatus::Won;
                    let num = self.guesses.len();
                    self.stats.record_win(num);
                    let msg = match num {
                        1 => "Genius! 🎉".to_string(),
                        2 => "Magnificent! 🎉".to_string(),
                        3 => "Impressive! 🎉".to_string(),
                        4 => "Splendid! 🎉".to_string(),
                        5 => "Great! 🎉".to_string(),
                        _ => "Phew! 😅".to_string(),
                    };
                    self.show_message(ctx, msg);
                    // Show stats after a short delay
                    let link = ctx.link().clone();
                    gloo::timers::callback::Timeout::new(2200, move || {
                        link.send_message(Msg::ShowStats);
                    })
                    .forget();
                } else if self.guesses.len() >= MAX_GUESSES {
                    self.status = GameStatus::Lost;
                    self.stats.record_loss();
                    let answer = self.answer.to_uppercase();
                    self.show_message(ctx, format!("The word was {}", answer));
                    let link = ctx.link().clone();
                    gloo::timers::callback::Timeout::new(2200, move || {
                        link.send_message(Msg::ShowStats);
                    })
                    .forget();
                }
                true
            }
            Msg::NewGame => {
                let new_answer = random_word();
                // Clear URL params if any
                let _ = window()
                    .history()
                    .and_then(|h| h.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(".")));
                self.answer = new_answer;
                self.current_guess.clear();
                self.guesses.clear();
                self.status = GameStatus::Playing;
                self.message = None;
                self.show_stats = false;
                self.is_shared_game = false;
                self.letter_states.clear();
                true
            }
            Msg::ShareUrl => {
                let url = self.build_share_url();
                // Try to use clipboard API
                let url_clone = url.clone();
                let link = ctx.link().clone();
                let clipboard = gloo::utils::window().navigator().clipboard();
                let promise = clipboard.write_text(&url_clone);
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    link.send_message(Msg::CopyDone);
                });
                self.show_message(ctx, "Link copied! 🔗".to_string());
                true
            }
            Msg::ToggleDifficulty => {
                if self.guesses.is_empty() {
                    self.difficulty = match self.difficulty {
                        Difficulty::Easy => Difficulty::Hard,
                        Difficulty::Hard => Difficulty::Easy,
                    };
                    let _ = LocalStorage::set(DIFFICULTY_KEY, &self.difficulty);
                } else {
                    self.show_message(
                        ctx,
                        "Cannot change difficulty mid-game".to_string(),
                    );
                }
                true
            }
            Msg::ShowStats => {
                self.show_stats = true;
                true
            }
            Msg::HideStats => {
                self.show_stats = false;
                true
            }
            Msg::ShowHowToPlay => {
                self.show_how_to_play = true;
                true
            }
            Msg::HideHowToPlay => {
                self.show_how_to_play = false;
                true
            }
            Msg::HideMessage => {
                self.message = None;
                true
            }
            Msg::CopyDone => {
                self.copy_done = false;
                true
            }
            Msg::ShowCreateLink => {
                self.show_create_link = true;
                self.create_link_word.clear();
                true
            }
            Msg::HideCreateLink => {
                self.show_create_link = false;
                self.create_link_word.clear();
                true
            }
            Msg::CreateLinkInput(raw) => {
                let filtered: String = raw
                    .chars()
                    .filter(|c| c.is_ascii_alphabetic())
                    .take(WORD_LENGTH)
                    .collect::<String>()
                    .to_lowercase();
                self.create_link_word = filtered;
                true
            }
            Msg::CopyCreateLink => {
                if !is_valid_word(&self.create_link_word) {
                    return false;
                }
                let url = Self::build_url_for_word(&self.create_link_word);
                let link = ctx.link().clone();
                let clipboard = gloo::utils::window().navigator().clipboard();
                let promise = clipboard.write_text(&url);
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    link.send_message(Msg::CopyDone);
                });
                self.show_message(ctx, "Challenge link copied! 🔗".to_string());
                true
            }
            Msg::DismissInstallBanner => {
                self.show_install_banner = false;
                let _ = LocalStorage::set(INSTALL_BANNER_KEY, true);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        // Keyboard handler
        let onkeydown = {
            let link = link.clone();
            Callback::from(move |e: KeyboardEvent| {
                let key = e.key();
                if key == "Enter" {
                    link.send_message(Msg::Enter);
                } else if key == "Backspace" {
                    link.send_message(Msg::Backspace);
                } else if key.len() == 1 && key.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                    link.send_message(Msg::KeyPress(key));
                }
            })
        };

        html! {
            <div class="app" tabindex="0" onkeydown={onkeydown}>
                { self.view_header(ctx) }
                { self.view_action_bar(ctx) }
                { self.view_message() }
                { self.view_board(ctx) }
                { self.view_keyboard(ctx) }
                { if self.show_stats { self.view_stats_modal(ctx) } else { html!{} } }
                { if self.show_how_to_play { self.view_how_to_play_modal(ctx) } else { html!{} } }
                { if self.show_create_link { self.view_create_link_modal(ctx) } else { html!{} } }
                { self.view_install_banner(ctx) }
            </div>
        }
    }
}

impl App {
    fn view_header(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <header class="header">
                <h1 class="title">{"Oh-My-Wordle!"}</h1>
            </header>
        }
    }

    fn view_action_bar(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let diff_label = match self.difficulty {
            Difficulty::Easy => "Easy",
            Difficulty::Hard => "Hard ★",
        };
        let diff_class = match self.difficulty {
            Difficulty::Easy => "btn-action",
            Difficulty::Hard => "btn-action btn-action-hard",
        };

        html! {
            <div class="action-bar">
                <button class="btn-action" onclick={link.callback(|_| Msg::NewGame)} title="New Game">
                    <span class="btn-action-icon">{"🔄"}</span>
                    <span class="btn-action-label">{"New"}</span>
                </button>
                <button class="btn-action" onclick={link.callback(|_| Msg::ShowStats)} title="Statistics">
                    <span class="btn-action-icon">{"📊"}</span>
                    <span class="btn-action-label">{"Stats"}</span>
                </button>
                <button class="btn-action" onclick={link.callback(|_| Msg::ShowHowToPlay)} title="How to play">
                    <span class="btn-action-icon">{"❓"}</span>
                    <span class="btn-action-label">{"How To"}</span>
                </button>
                <button class="btn-action" onclick={link.callback(|_| Msg::ShowCreateLink)} title="Create challenge">
                    <span class="btn-action-icon">{"✏️"}</span>
                    <span class="btn-action-label">{"Create"}</span>
                </button>
                <button class="btn-action" onclick={link.callback(|_| Msg::ShareUrl)} title="Share this puzzle">
                    <span class="btn-action-icon">{"🔗"}</span>
                    <span class="btn-action-label">{"Share"}</span>
                </button>
                <button class={diff_class} onclick={link.callback(|_| Msg::ToggleDifficulty)} title="Toggle difficulty (only at game start)">
                    <span class="btn-action-icon">{"⚙️"}</span>
                    <span class="btn-action-label">{ diff_label }</span>
                </button>
                <a class="btn-action" href="https://buymeacoffee.com/nabeelvandayar" target="_blank" rel="noopener noreferrer" title="Buy Me a Coffee">
                    <span class="btn-action-icon">{"☕"}</span>
                    <span class="btn-action-label">{"Coffee"}</span>
                </a>
            </div>
        }
    }

    fn view_message(&self) -> Html {
        if let Some(ref msg) = self.message {
            html! {
                <div class="message">{ msg }</div>
            }
        } else {
            html! {}
        }
    }

    fn view_board(&self, _ctx: &Context<Self>) -> Html {
        let mut rows = Vec::new();

        // Submitted guesses
        for row in &self.guesses {
            let tiles = row.word.chars().enumerate().map(|(i, ch)| {
                let state = row.states.get(i).cloned().unwrap_or(TileState::Absent);
                let cls = format!("tile {}", Self::letter_state_class(&state));
                html! {
                    <div class={cls}>
                        <span>{ ch.to_uppercase().to_string() }</span>
                    </div>
                }
            }).collect::<Html>();
            rows.push(html! {
                <div class="row submitted">{ tiles }</div>
            });
        }

        // Current guess row
        if self.guesses.len() < MAX_GUESSES && self.status == GameStatus::Playing {
            let tiles: Html = (0..WORD_LENGTH).map(|i| {
                let ch = self.current_guess.chars().nth(i);
                let (letter, cls) = match ch {
                    Some(c) => (c.to_uppercase().to_string(), "tile filled".to_string()),
                    None => (" ".to_string(), "tile".to_string()),
                };
                html! {
                    <div class={cls}>
                        <span>{ letter }</span>
                    </div>
                }
            }).collect();
            rows.push(html! {
                <div class="row current">{ tiles }</div>
            });
        }

        // Empty rows
        let empty_rows_needed = MAX_GUESSES.saturating_sub(rows.len());
        for _ in 0..empty_rows_needed {
            let tiles: Html = (0..WORD_LENGTH).map(|_| {
                html! { <div class="tile"><span>{" "}</span></div> }
            }).collect();
            rows.push(html! {
                <div class="row empty">{ tiles }</div>
            });
        }

        html! {
            <div class="board-container">
                <div class="board">
                    { for rows.into_iter() }
                </div>
            </div>
        }
    }

    fn view_keyboard(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let rows = [
            vec!['q','w','e','r','t','y','u','i','o','p'],
            vec!['a','s','d','f','g','h','j','k','l'],
            vec!['z','x','c','v','b','n','m'],
        ];

        let key_rows: Html = rows.iter().enumerate().map(|(row_idx, row)| {
            let keys: Html = row.iter().map(|&ch| {
                let state = self.letter_states.get(&ch);
                let cls = match state {
                    Some(TileState::Correct) => "key correct",
                    Some(TileState::Present) => "key present",
                    Some(TileState::Absent) => "key absent",
                    _ => "key",
                };
                let key_str = ch.to_string();
                let link2 = link.clone();
                html! {
                    <button class={cls}
                        onclick={Callback::from(move |_| link2.send_message(Msg::KeyPress(key_str.clone())))}>
                        { ch.to_uppercase().to_string() }
                    </button>
                }
            }).collect();

            let enter_backspace = if row_idx == 2 {
                let link_e = link.clone();
                let link_b = link.clone();
                html! {
                    <>
                        <button class="key key-wide"
                            onclick={Callback::from(move |_| link_e.send_message(Msg::Enter))}>
                            {"Enter"}
                        </button>
                        { keys }
                        <button class="key key-wide"
                            onclick={Callback::from(move |_| link_b.send_message(Msg::Backspace))}>
                            {"⌫"}
                        </button>
                    </>
                }
            } else {
                keys
            };

            html! {
                <div class="keyboard-row">{ enter_backspace }</div>
            }
        }).collect();

        html! {
            <div class="keyboard">{ key_rows }</div>
        }
    }

    fn view_stats_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let stats = &self.stats;
        let max_dist = stats.guess_distribution.iter().max().copied().unwrap_or(1).max(1);

        let dist_bars: Html = stats.guess_distribution.iter().enumerate().map(|(i, &count)| {
            let pct = ((count as f32 / max_dist as f32) * 100.0) as u32;
            let pct = pct.max(if count > 0 { 8 } else { 0 });
            let is_current = self.status == GameStatus::Won
                && self.guesses.len() == i + 1;
            let bar_class = if is_current { "dist-bar current" } else { "dist-bar" };
            html! {
                <div class="dist-row">
                    <span class="dist-label">{ (i + 1).to_string() }</span>
                    <div class={bar_class} style={format!("width: {}%", pct)}>
                        <span>{ count.to_string() }</span>
                    </div>
                </div>
            }
        }).collect();

        let share_section = if self.status != GameStatus::Playing {
            let share_url = self.build_share_url();
            let link2 = link.clone();
            html! {
                <div class="share-section">
                    <p class="share-label">{"Share this puzzle:"}</p>
                    <input class="share-url" readonly=true value={share_url} />
                    <button class="btn-share" onclick={Callback::from(move |_| link2.send_message(Msg::ShareUrl))}>
                        {"Copy Link"}
                    </button>
                </div>
            }
        } else {
            html! {}
        };

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideStats)}>
                <div class="modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <button class="modal-close" onclick={link.callback(|_| Msg::HideStats)}>{"✕"}</button>
                    <h2>{"Statistics"}</h2>
                    <div class="stats-row">
                        <div class="stat">
                            <span class="stat-number">{ stats.played.to_string() }</span>
                            <span class="stat-label">{"Played"}</span>
                        </div>
                        <div class="stat">
                            <span class="stat-number">{ stats.win_percentage().to_string() }</span>
                            <span class="stat-label">{"Win %"}</span>
                        </div>
                        <div class="stat">
                            <span class="stat-number">{ stats.current_streak.to_string() }</span>
                            <span class="stat-label">{"Current Streak"}</span>
                        </div>
                        <div class="stat">
                            <span class="stat-number">{ stats.max_streak.to_string() }</span>
                            <span class="stat-label">{"Max Streak"}</span>
                        </div>
                    </div>
                    <h3>{"Guess Distribution"}</h3>
                    <div class="dist-chart">
                        { dist_bars }
                    </div>
                    { share_section }
                    <button class="btn-new-game" onclick={link.callback(|_| Msg::NewGame)}>
                        {"New Game"}
                    </button>
                </div>
            </div>
        }
    }

    fn view_how_to_play_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideHowToPlay)}>
                <div class="modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <button class="modal-close" onclick={link.callback(|_| Msg::HideHowToPlay)}>{"✕"}</button>
                    <h2>{"How to Play"}</h2>

                    <div class="how-to-section">
                        <h3>{"Goal"}</h3>
                        <p>{"Guess the hidden 5-letter word in up to 6 tries."}</p>
                    </div>

                    <div class="how-to-section">
                        <h3>{"Tile Colors"}</h3>
                        <p>{"🟩 Green: right letter, right spot."}</p>
                        <p>{"🟨 Yellow: right letter, wrong spot."}</p>
                        <p>{"⬛ Gray: letter is not in the word."}</p>
                    </div>

                    <div class="how-to-section">
                        <h3>{"Controls"}</h3>
                        <p>{"Type on your keyboard or tap the on-screen keys."}</p>
                        <p>{"Press Enter to submit and ⌫ to delete."}</p>
                    </div>

                    <div class="how-to-section">
                        <h3>{"Game Buttons"}</h3>
                        <p>{"🔄 New: start a new random puzzle."}</p>
                        <p>{"📊 Stats: view wins, streaks, and guess distribution."}</p>
                        <p>{"✏️ Create: make a challenge link with a chosen valid word."}</p>
                        <p>{"🔗 Share: copy a link to your current puzzle."}</p>
                        <p>{"⚙️ Easy / Hard: switch difficulty before your first guess."}</p>
                    </div>

                    <div class="how-to-section">
                        <h3>{"Hard Mode"}</h3>
                        <p>{"In Hard mode, all revealed clues must be reused in future guesses."}</p>
                    </div>
                </div>
            </div>
        }
    }

    fn view_create_link_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let word = &self.create_link_word;

        let is_valid = word.len() == WORD_LENGTH && is_valid_word(word);
        let error_msg: Option<&str> = if !word.is_empty() && word.len() < WORD_LENGTH {
            Some("Word must be 5 letters")
        } else if word.len() == WORD_LENGTH && !is_valid {
            Some("Not in word list")
        } else {
            None
        };
        let generated_url = if is_valid {
            Some(Self::build_url_for_word(word))
        } else {
            None
        };

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideCreateLink)}>
                <div class="modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <button class="modal-close" onclick={link.callback(|_| Msg::HideCreateLink)}>{"✕"}</button>
                    <h2>{"Create Challenge"}</h2>
                    <p class="create-link-desc">{"Type any valid 5-letter word to create a shareable puzzle link for a friend:"}</p>
                    <input
                        class="create-link-input"
                        type="text"
                        placeholder="WORD"
                        maxlength="5"
                        value={word.to_uppercase()}
                        oninput={link.callback(|e: InputEvent| {
                            let input: web_sys::HtmlInputElement = e.target().unwrap().unchecked_into();
                            Msg::CreateLinkInput(input.value())
                        })}
                    />
                    if let Some(err) = error_msg {
                        <p class="create-link-error">{ err }</p>
                    }
                    if let Some(ref url) = generated_url {
                        <div class="create-link-url-section">
                            <input class="share-url" readonly=true value={url.clone()} />
                            <button class="btn-share" onclick={link.callback(|_| Msg::CopyCreateLink)}>
                                {"Copy Challenge Link"}
                            </button>
                        </div>
                    }
                </div>
            </div>
        }
    }

    fn view_install_banner(&self, ctx: &Context<Self>) -> Html {
        if !self.show_install_banner {
            return html! {};
        }
        let Some((icon, instructions)) = get_install_instructions() else {
            return html! {};
        };
        let link = ctx.link();
        html! {
            <div class="install-banner">
                <span class="install-banner-icon">{ icon }</span>
                <div class="install-banner-content">
                    <p class="install-banner-title">{"Add to Home Screen"}</p>
                    <p class="install-banner-text">{ instructions }</p>
                </div>
                <button
                    class="install-banner-close"
                    onclick={link.callback(|_| Msg::DismissInstallBanner)}
                    aria-label="Dismiss"
                >{"✕"}</button>
            </div>
        }
    }
}
