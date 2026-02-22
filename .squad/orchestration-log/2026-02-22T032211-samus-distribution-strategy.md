# Orchestration Log: Samus (Distribution Strategy Research)

**Timestamp:** 2026-02-22T03:22:11Z  
**Agent:** Samus (Lead)  
**Task:** Distribution Strategy Research & Analysis  
**Mode:** Background  
**Model:** claude-sonnet-4.5

## Summary

Comprehensive research and analysis of distribution channels for citrust targeting non-technical users, especially SteamOS/Steam Deck gamers.

## Recommended Channels (Ranked by Priority)

### 🟢 Priority #1: Flatpak / Flathub
- **Target:** SteamOS, Steam Deck, Fedora, Linux Mint
- **Effort:** Medium (~1–2 days)
- **Impact:** Largest audience; one-click install via Discover
- **Special:** SteamOS native standard; users can add to Steam for Gaming Mode

### 🟢 Priority #2: AppImage
- **Target:** SteamOS, Linux desktops, Steam Deck portability
- **Effort:** Low–Medium (~4–8 hours)
- **Impact:** Zero-install UX; portable across all distros
- **Special:** Can be added to Steam as non-Steam game

### 🟢 Priority #2b: Windows (winget)
- **Target:** Windows gamers, existing GitHub Releases users
- **Effort:** Low (~2–4 hours)
- **Impact:** Users discover via `winget search citrust`
- **Special:** Leverages existing `.exe` releases; handles auto-updates

### 🟡 Priority #3: AUR (Arch User Repository)
- **Target:** Arch/SteamOS power users
- **Effort:** Low (~2–4 hours)
- **Impact:** Niche; limited by SteamOS read-only rootfs
- **Special:** Auto-updates via `yay -Syu`

### 🟡 Priority #4: Scoop (Windows)
- **Target:** Windows developers
- **Effort:** Very Low (~1 hour)
- **Impact:** Marginal; low effort
- **Special:** Decentralized bucket system

### 🟡 Priority #5: Homebrew (macOS/Linux)
- **Target:** macOS users, Homebrew enthusiasts
- **Effort:** Low–Medium (~4–8 hours)
- **Impact:** Niche macOS audience
- **Special:** Defer unless explicit demand

### 🔴 Rejected: Snap
- SteamOS doesn't use Snap; inferior to Flatpak
- Skip indefinitely

## Implementation Roadmap

**Phase 1 (Week 1–2):** Flatpak manifest, AppStream metadata, icon, desktop file  
**Phase 2 (Week 2–3):** AppImage CI/CD + winget manifests (parallel)  
**Phase 3 (Week 4+):** AUR + Scoop (if demand)  

## Deliverables

Detailed analysis document with:
- Requirements for each channel (manifests, metadata, icons)
- Effort estimates and effort/ROI analysis
- SteamOS-specific considerations (gamepad, read-only rootfs, Flathub access)
- Resource links and tutorial references
- Timeline and next-steps checklist

## Key Insights

1. **Flatpak is table-stakes** for SteamOS/Steam Deck adoption
2. **AppImage is essential** for portability and zero-install UX
3. **Gamepad support** works seamlessly once sandboxing permissions are correct
4. **SteamOS read-only rootfs** eliminates AUR as primary channel; Flatpak/AppImage are only beginner-friendly options
5. **Windows gamers** can be served via winget with minimal effort (already have `.exe`)

## Output

Complete decision document written to `.squad/decisions/inbox/samus-distribution-strategy.md` (360+ lines, 12KB).
