# Skill: egui Background Threading Pattern

**Domain:** GUI Development  
**Framework:** egui/eframe  
**Pattern:** Non-blocking UI with background processing  
**Author:** Fox

## Problem

egui is immediate-mode and runs on the main UI thread. Long-running operations (file I/O, encryption, network requests) will freeze the UI if executed directly in the `update()` method.

## Solution

Use `std::sync::mpsc` channels to communicate between a background thread and the UI thread. The background thread performs the long-running work and sends progress updates via a channel. The UI thread polls the channel each frame and updates the UI state accordingly.

## Pattern

```rust
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Debug, Clone)]
enum ProgressMessage {
    Started,
    Update(String),
    Done,
    Error(String),
}

struct AppState {
    rx: Receiver<ProgressMessage>,
    progress_messages: Vec<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for messages (non-blocking)
        while let Ok(msg) = self.state.rx.try_recv() {
            match msg {
                ProgressMessage::Update(text) => {
                    self.state.progress_messages.push(text);
                }
                ProgressMessage::Done => {
                    // Handle completion
                }
                // ... other cases
            }
        }
        
        // Render UI
        egui::CentralPanel::default().show(ctx, |ui| {
            for msg in &self.state.progress_messages {
                ui.label(msg);
            }
        });
        
        // Request repaint to poll channel again next frame
        ctx.request_repaint();
    }
}
```

## Borrow Checker Gotcha

**Problem:** Cannot modify `self.state` while borrowing it mutably in the receiver loop.

**Solution:** Collect state changes in local variables, apply after releasing the borrow. See citrust `src/gui.rs` for full example.

## Key Considerations

1. **Non-blocking:** Use `try_recv()` (not `recv()`) to avoid blocking the UI thread
2. **Repaint requests:** Call `ctx.request_repaint()` to poll the channel every frame
3. **Message backpressure:** If messages arrive faster than UI can render, they queue in the channel
4. **Error handling:** Sender drops when thread exits, causing `try_recv()` to return `Err(Disconnected)`
5. **Thread safety:** Messages must be `Send` (automatic for most types)

## References

- egui threading FAQ: https://docs.rs/egui/latest/egui/#threading
- citrust GUI implementation: `src/gui.rs`
- Rust mpsc docs: https://doc.rust-lang.org/std/sync/mpsc/
