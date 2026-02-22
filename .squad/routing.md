# Routing Rules

## Domain Routing

| Domain | Primary | Backup |
|--------|---------|--------|
| Architecture, scope, decisions | Samus | — |
| Rust code, crypto, binary parsing, CLI | Link | Samus |
| GUI, frontend, SteamOS UI, UX | Fox | Link |
| Tests, QA, edge cases, correctness | Toad | Link |
| Logs, decisions, memory | Scribe | — |
| Work queue, backlog monitoring | Ralph | — |

## Keyword Routing

| Keywords | Route to |
|----------|----------|
| decrypt, AES, CTR, crypto, key, NCSD, NCCH, partition, binary, parse, struct, ROM | Link |
| GUI, UI, window, button, SteamOS, frontend, iced, egui, component, theme | Fox |
| test, assert, edge case, coverage, correctness, verify, validate | Toad |
| architecture, design, review, scope, plan, build, publish, release, package, deploy | Samus |
| status, backlog, issues, board, monitor | Ralph |

## Code Review

- All PRs reviewed by Samus (Lead)
- Crypto code additionally reviewed by Toad (correctness tests required)
