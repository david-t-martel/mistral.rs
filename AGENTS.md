# Repository Guidelines

## Project Structure & Module Organization

- `mistralrs/` exposes the public Rust API for text and multimodal inference; lower-level kernels live in `mistralrs-core/`.
- Domain-specific crates such as `mistralrs-quant/`, `mistralrs-vision/`, and `mistralrs-paged-attn/` house quantization, vision, and paged-attention logic.
- Server code resides in `mistralrs-server/` with shared components under `mistralrs-server-core/`; web assets live in `mistralrs-web-chat/`.
- End-to-end tests sit in `tests/`; docs, examples, and scripts live under `docs/`, `examples/`, and `scripts/` respectively. Sync walkthroughs when changing behavior.

## Build, Test, and Development Commands

- `cargo check --workspace` validates all crates quickly; run before every PR.
- `cargo build --workspace --release --features "<feature list>"` builds optimized binaries (e.g. `cargo build --workspace --release --features "cuda flash-attn"`).
- `cargo fmt --all` and `cargo clippy --workspace --tests --examples -- -D warnings` enforce formatting and linting—fix findings, do not suppress.
- Launch the server with `./target/release/mistralrs-server run -m <model>` once built; add flags for accelerator-specific paths.

## Coding Style & Naming Conventions

- Rust follows edition 2021 with 4-space indentation, `snake_case` functions, and `CamelCase` types.
- Keep Cargo feature names lowercase with hyphens; mirror upstream model names when adding assets.
- Python components (`examples/python/`, `mistralrs-pyo3/`) follow PEP 8; run `make fmt` when touching cross-language code.

## Testing Guidelines

- Prefer unit tests colocated with sources; integration and smoke flows stay in `tests/`.
- Core regression: `cargo test -p mistralrs-core -p mistralrs-quant -p mistralrs-vision`; full sweep: `cargo test --workspace`.
- Gate GPU-dependent cases behind features; set `HF_TOKEN` or `TESTS_HF_TOKEN` before tests that download models.
- Store new fixtures under `tests/fixtures/` and document large artifacts in `docs/`.

## Prime Directive: Critical Skepticism

- Treat every change as broken until proven otherwise; verify with real runs before claiming success (`cargo test`, custom scripts, or reproduced commands).
- Inspect your own diffs for edge cases and regression risks prior to review; rerun impacted suites if assumptions change.
- When verification is impossible, clearly note the gap in your PR/summary and list what remains untested.

## Commit & Pull Request Guidelines

- Use Conventional Commits scoped to the crate, e.g., `feat(mistralrs-core): enable paged attention`.
- PRs should explain motivation, user-facing impact, and feature flags touched; list commands executed (e.g., `cargo check`, `cargo test`).
- Link related issues, attach screenshots/logs for server or CLI changes, and note follow-up items as checklists.

## Security & Configuration Tips

- Never commit API tokens or model weights; keep secrets in `.env` (gitignored) and verify layout via `model.safetensors.index.json`.
- When adding models or quant paths, confirm Candle VarBuilder prefixes match PyTorch checkpoints and record deviations in `docs/`.
- Review agent playbooks (`CLAUDE.md`, `MISTRAL_AGENT_SETUP.md`, etc.) before modifying automation flows to keep integration consistent.
