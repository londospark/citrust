# Decision: GitHub Project Management Structure

**Author:** Samus (Lead)  
**Date:** 2026-02-22  
**Status:** Implemented

## Context

The citrust repo (londospark/citrust) needed full GitHub project management infrastructure to coordinate the 4-phase Rust port of b3DSDecrypt.py.

## Decisions

### 1. Label Taxonomy
Three-axis labeling: `phase:*` (4 phases), `module:*` (7 modules), `type:*` (4 types), plus `squad:*` labels for agent assignment. This enables filtering by any combination of phase, module, and assignee.

### 2. Issue Granularity
21 issues total — one per module per phase, plus cross-cutting test/benchmark/infra issues. Each issue has acceptance criteria, dependency references, and clear scope. Not too granular (avoids overhead), not too coarse (enables parallel work).

### 3. Milestone-per-Phase
Four milestones matching the four phases. Issue #21 (CI) left unassigned to a milestone since it spans all phases. Each milestone has a clear deliverable definition.

### 4. Project Board
Single "citrust Roadmap" board (project #6) with all 21 issues. Uses GitHub Projects v2 default columns (Todo/In Progress/Done) which is sufficient for a small team.

### 5. Agent Assignment
- **Link** (Core Dev): #1–#6 (Phase 1 implementation), #10–#11 (Phase 2 SIMD), #13–#15 (Phase 3 threading)
- **Toad** (Tester): #7–#9 (Phase 1 tests/benchmarks), #12 (Phase 2 regression), #16 (Phase 3 regression)
- **Fox** (GUI Dev): #18–#19 (Phase 4 GUI + gamepad)
- **Samus** (Lead): #17 (workspace split), #20 (AppImage), #21 (CI)

## Issue Reference

| # | Title | Phase | Owner |
|---|-------|-------|-------|
| 1 | keys.rs — key constants and derivation | 1 | Link |
| 2 | crypto.rs — ROL128 + AES-CTR wrapper | 1 | Link |
| 3 | ncsd.rs — NCSD header parser | 1 | Link |
| 4 | ncch.rs — NCCH partition parser | 1 | Link |
| 5 | decrypt.rs — decryption orchestrator | 1 | Link |
| 6 | main.rs — CLI entry point | 1 | Link |
| 7 | Unit tests for all Phase 1 modules | 1 | Toad |
| 8 | Integration tests — round-trip decryption | 1 | Toad |
| 9 | Benchmark Phase 1 vs Python | 1 | Toad |
| 10 | Enable AES-NI via .cargo/config.toml | 2 | Link |
| 11 | Profile and tune AES-CTR chunk sizes | 2 | Link |
| 12 | Regression tests + benchmark Phase 1 vs Phase 2 | 2 | Toad |
| 13 | Refactor decrypt.rs for parallel I/O | 3 | Link |
| 14 | Chunk-level RomFS parallelism with rayon | 3 | Link |
| 15 | Section-level parallelism | 3 | Link |
| 16 | Regression tests + benchmark Phase 2 vs Phase 3 | 3 | Toad |
| 17 | Convert to workspace (core/cli/gui crates) | 4 | Samus |
| 18 | egui/eframe GUI — file picker, progress, done | 4 | Fox |
| 19 | Gamepad input support with gilrs | 4 | Fox |
| 20 | AppImage packaging for SteamOS | 4 | Samus |
| 21 | GitHub Actions CI workflow | — | Samus |
