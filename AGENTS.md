# Repository Guidelines

## Project Structure & Module Organization

- `mistralrs/` exposes the public Rust API for text and multimodal inference; lower-level kernels live in `mistralrs-core/`, while vision, quantization, and paged-attention logic sit in sibling crates such as `mistralrs-vision/`, `mistralrs-quant/`, and `mistralrs-paged-attn/`.
- The CLI and OpenAI-compatible server code resides in `mistralrs-server/`, with shared logic extracted to `mistralrs-server-core/`; web assets are under `mistralrs-web-chat/`.
- End-to-end tests and smoke scenarios live in the top-level `tests/` directory; crate-specific unit tests are colocated with sources.
- Documentation, examples, and utility scripts are in `docs/`, `examples/`, and `scripts/` respectively—keep walkthroughs and notebooks synchronized with code changes.

## Build, Test, and Development Commands

- `cargo check --workspace` validates the entire workspace quickly; run it before opening a PR.
- `cargo build --workspace --release --features "<features>"` produces optimized binaries; use feature flags such as `cuda`, `flash-attn`, or `metal` when targeting specific accelerators.
- `cargo fmt --all` and `cargo clippy --workspace --tests --examples -- -D warnings` enforce formatting and lint standards; fix issues rather than silencing them.
- Core regression suite: `cargo test -p mistralrs-core -p mistralrs-quant -p mistralrs-vision`; full coverage: `cargo test --workspace`.
- Launch the server locally with `./target/release/mistralrs-server run -m <model> [flags]` after building.

## Coding Style & Naming Conventions

- Rust code follows the 2021 edition with 4-space indentation, `snake_case` functions, `CamelCase` types, and module-level docs summarizing behavior.
- Python bindings/scripts (under `examples/python/` and `mistralrs-pyo3/`) follow PEP 8; run `make fmt` if editing cross-language components.
- Keep feature flags and Cargo features lowercase and hyphen-separated; mirror upstream model names when adding assets.

## Testing Guidelines

- Prefer focused unit tests near the code they exercise; integration flows belong in `tests/`.
- Name Rust tests descriptively (e.g., `it_loads_awq_weights`) and gate GPU-dependent cases behind feature flags.
- Set `HF_TOKEN` or `TESTS_HF_TOKEN` in the environment before invoking suites that download models.
- Record any new golden data under `tests/fixtures/` and document large artifacts in `docs/`.

## Commit & Pull Request Guidelines

- Use Conventional Commit prefixes scoped to the crate, such as `feat(mistralrs-core): enable paged attention` or `fix(server): guard empty prompts`.
- Each PR should describe motivation, summarize user-facing impact, list feature flags touched, and note which commands were run (e.g., `cargo check`, `cargo test`).
- Cross-link issues and include screenshots or logs when altering CLI/server behavior; highlight follow-up work in a checklist.

## Related Playbooks & Onboarding

- Onboarding & quickstarts: [`README.md`](README.md), [`SETUP_COMPLETE.md`](SETUP_COMPLETE.md), [`DEPLOYMENT_QUICK_START.md`](DEPLOYMENT_QUICK_START.md), [`MAKEFILE_TARGETS_SUMMARY.md`](MAKEFILE_TARGETS_SUMMARY.md).
- Agent playbooks: [`CLAUDE.md`](CLAUDE.md), [`AUTO_CLAUDE_SETUP.md`](AUTO_CLAUDE_SETUP.md), [`MISTRAL_AGENT_SETUP.md`](MISTRAL_AGENT_SETUP.md), [`AGENT_IMPLEMENTATION_SUMMARY.md`](AGENT_IMPLEMENTATION_SUMMARY.md), [`AGENT_INTEGRATION_ANALYSIS.md`](AGENT_INTEGRATION_ANALYSIS.md), [`AGENT_MODE_IMPLEMENTATION.md`](AGENT_MODE_IMPLEMENTATION.md), [`AGENT_TOOLS_INTEGRATION.md`](AGENT_TOOLS_INTEGRATION.md), [`AGENT_TOOLS_SESSION_SUMMARY.md`](AGENT_TOOLS_SESSION_SUMMARY.md), [`REACT_AGENT_ANALYSIS_AND_FIX.md`](REACT_AGENT_ANALYSIS_AND_FIX.md), [`REACT_AGENT_FIX.md`](REACT_AGENT_FIX.md).
- Automation & tooling: [`warp.md`](warp.md), [`.github/copilot-instructions.md`](.github/copilot-instructions.md), [`CI_CD_SETUP_SUMMARY.md`](CI_CD_SETUP_SUMMARY.md), [`COVERAGE_FIX_STATUS.md`](COVERAGE_FIX_STATUS.md).
- Roadmaps & health reports: [`PHASE1_IMPLEMENTATION_PLAN.md`](PHASE1_IMPLEMENTATION_PLAN.md), [`PHASE1_PROGRESS.md`](PHASE1_PROGRESS.md), [`QUICK_WINS_COMPLETE.md`](QUICK_WINS_COMPLETE.md), [`FEATURE_STABILIZATION.md`](FEATURE_STABILIZATION.md), [`PERFORMANCE_OPTIMIZATION_COMPLETE.md`](PERFORMANCE_OPTIMIZATION_COMPLETE.md), [`DEPLOYMENT_CHECKLIST_VALIDATED.md`](DEPLOYMENT_CHECKLIST_VALIDATED.md), [`DEPLOYMENT_IMPLEMENTATION_COMPLETE.md`](DEPLOYMENT_IMPLEMENTATION_COMPLETE.md), [`DEPLOYMENT_SUMMARY.md`](DEPLOYMENT_SUMMARY.md), [`DEPLOYMENT.md`](DEPLOYMENT.md).

## Environment & Security Notes

- Never commit API tokens or model weights; rely on `.env` files ignored by git and reference `model.safetensors.index.json` to verify layout.
- When introducing new models or quantization paths, confirm Candle VarBuilder prefixes match PyTorch checkpoints and document deviations in `docs/`.
