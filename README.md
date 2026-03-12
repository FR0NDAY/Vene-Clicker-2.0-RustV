# VeneClicker 2.0 (Rust)
Rust-only rewrite of VeneClicker. Java/Maven sources were removed.

## Requirements
- Windows
- Rust toolchain (`rustup`, `cargo`)

## Run
```bash
cargo run --release
```

## Build
```bash
cargo build --release
```

## Notes
- Config is stored in `config.txt`.
- Global key/mouse hooks are handled in Rust (`rdev`).
- Clicking and active-window checks use WinAPI via `windows-sys`.
- FOR MINECRAFT JAVA, YOU MUST Tggle Raw Input or it will just move you to the top left !
