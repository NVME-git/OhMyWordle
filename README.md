# OhMyWordle! 🟩

A Wordle clone built with **Rust + WebAssembly** (Yew framework), hosted on GitHub Pages.

[![Buy Me A Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://buymeacoffee.com/nabeelvandayar)

![OhMyWordle Screenshot](https://github.com/user-attachments/assets/b8df1de6-9ce6-4204-8c86-896f8bbe251f)

## Features

- 🎲 **Random word each session** — a new 5-letter word every time you play (not daily)
- 🔗 **URL sharing** — generate a shareable link with your word encoded in the URL; friends who open it play the same puzzle
- 🌟 **Easy / Hard mode** — Hard mode requires you to reuse all revealed green/yellow letters in subsequent guesses
- 📊 **Stats tracking** — win %, current streak, max streak, and guess distribution stored in your browser's `localStorage`
- ⌨️ **On-screen keyboard** — click or use your physical keyboard
- 🎨 **Dark theme** — styled to match the classic Wordle look and feel

## How to Play

1. Guess a 5-letter word in 6 tries.
2. After each guess, tiles change color:
   - 🟩 **Green** — correct letter, correct position
   - 🟨 **Yellow** — correct letter, wrong position
   - ⬛ **Gray** — letter not in the word
3. Use the 🔗 button to copy a shareable link for your current puzzle.
4. Use the 🔄 button to start a new random game.
5. Toggle **Easy / Hard** mode with the button in the top-left (before making your first guess).

## Tech Stack

| Layer | Technology |
|---|---|
| Language | [Rust](https://www.rust-lang.org/) |
| UI Framework | [Yew](https://yew.rs/) (React-like, compiles to WASM) |
| Build Tool | [Trunk](https://trunkrs.dev/) |
| Hosting | [GitHub Pages](https://pages.github.com/) |
| State Persistence | Browser `localStorage` |

## Local Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Trunk
cargo install trunk

# Clone and run dev server
git clone https://github.com/NVME-git/OhMyWordle
cd OhMyWordle
trunk serve
# Open http://localhost:8080
```

## Building for Production

```bash
trunk build --release --public-url /OhMyWordle/
# Output in ./dist/
```

## Deployment

GitHub Actions automatically builds and deploys to GitHub Pages on every push to `main`.
See [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml).

## URL Sharing

When you click the 🔗 button, the current puzzle word is Base64-encoded and appended as a query parameter (`?w=<encoded>`). Anyone who opens that URL will play with the same word.

Example: `https://nvme-git.github.io/OhMyWordle/?w=YnJhdmU` sets the word to `brave`.
