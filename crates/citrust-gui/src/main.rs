use citrust_core::keydb::KeyDatabase;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Instant;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("citrust — 3DS ROM Decrypter"),
        ..Default::default()
    };

    eframe::run_native(
        "citrust",
        options,
        Box::new(|cc| {
            // Configure large fonts for gamepad-friendly UI
            let mut style = (*cc.egui_ctx.style()).clone();
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(48.0));
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(24.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(28.0));
            style.text_styles.insert(
                egui::TextStyle::Name("Small".into()),
                egui::FontId::proportional(16.0),
            );
            cc.egui_ctx.set_style(style);

            Ok(Box::new(CitrustApp::default()))
        }),
    )
}

#[derive(Debug, Clone)]
enum ProgressMessage {
    Started,
    Update(String),
    Done,
    Error(String),
}

enum Screen {
    KeySetup,
    SelectFile,
    Decrypting,
    Done,
}

struct DecryptState {
    file_path: PathBuf,
    progress_messages: Vec<String>,
    current_section: String,
    encryption_method: Option<String>,
    start_time: Instant,
    rx: Receiver<ProgressMessage>,
}

struct DoneState {
    duration_secs: u64,
}

struct CitrustApp {
    screen: Screen,
    selected_file: Option<PathBuf>,
    decrypt_state: Option<DecryptState>,
    done_state: Option<DoneState>,
    keydb: Option<KeyDatabase>,
    key_status: String,
    key_save_message: Option<String>,
}

impl Default for CitrustApp {
    fn default() -> Self {
        let (keydb, key_status, screen) = match KeyDatabase::search_default_locations()
            .and_then(|path| KeyDatabase::from_file(&path).ok())
        {
            Some(db) => {
                let status = format!("🔑 Keys loaded ({} keys)", db.len());
                (Some(db), status, Screen::SelectFile)
            }
            None => (None, String::new(), Screen::KeySetup),
        };

        Self {
            screen,
            selected_file: None,
            decrypt_state: None,
            done_state: None,
            keydb,
            key_status,
            key_save_message: None,
        }
    }
}

impl CitrustApp {
    fn show_key_setup_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);

            ui.heading("🔑 Key Setup Required");
            ui.add_space(40.0);

            ui.label("citrust needs an aes_keys.txt file to decrypt 3DS ROMs.");
            ui.add_space(60.0);

            let button_size = egui::vec2(400.0, 80.0);
            if ui
                .add_sized(button_size, egui::Button::new("📁 Browse for Key File"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Key File", &["txt"])
                    .set_title("Select aes_keys.txt")
                    .pick_file()
            {
                self.load_and_save_keys(&path);
            }

            // Show error if invalid file was selected
            if let Some(msg) = &self.key_save_message
                && msg.starts_with('❌')
            {
                ui.add_space(20.0);
                ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(220, 80, 80)));
            }

            ui.add_space(30.0);

            ui.label(
                egui::RichText::new("You can dump keys from your 3DS using GodMode9")
                    .text_style(egui::TextStyle::Name("Small".into()))
                    .color(egui::Color32::from_gray(140)),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("See README for setup instructions")
                    .text_style(egui::TextStyle::Name("Small".into()))
                    .color(egui::Color32::from_gray(120)),
            );
        });

        ctx.request_repaint();
    }

    fn show_select_file_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);

            ui.heading("citrust — 3DS ROM Decrypter");
            ui.add_space(60.0);

            // Large "Select ROM File" button
            let button_size = egui::vec2(400.0, 80.0);
            if ui
                .add_sized(button_size, egui::Button::new("📁 Select ROM File"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("3DS ROM", &["3ds"])
                    .set_title("Select 3DS ROM to Decrypt")
                    .pick_file()
            {
                self.selected_file = Some(path);
            }

            ui.add_space(30.0);

            // Show selected file if available
            if let Some(path) = &self.selected_file {
                ui.label(format!("Selected: {}", path.display()));
                ui.add_space(20.0);

                // Large "Decrypt" button appears after file selection
                if ui
                    .add_sized(button_size, egui::Button::new("🔓 Decrypt"))
                    .clicked()
                {
                    self.start_decryption(path.clone());
                }
            }

            // Show key save toast if present
            if let Some(msg) = &self.key_save_message {
                ui.add_space(30.0);
                ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(100, 200, 100)));
            }
        });

        ctx.request_repaint();
    }

    fn show_decrypting_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Clear toast on screen transition
        self.key_save_message = None;

        // Poll for progress messages and collect state changes
        let mut should_complete = false;
        let mut completion_duration = 0u64;

        if let Some(state) = &mut self.decrypt_state {
            while let Ok(msg) = state.rx.try_recv() {
                match msg {
                    ProgressMessage::Started => {
                        state.progress_messages.clear();
                    }
                    ProgressMessage::Update(text) => {
                        // Extract encryption method from first message
                        if text.starts_with("Encryption Method:") {
                            state.encryption_method = Some(text.clone());
                        }
                        // Track current section
                        if text.contains("ExeFS") || text.contains("RomFS") {
                            state.current_section = text.clone();
                        }
                        state.progress_messages.push(text);
                    }
                    ProgressMessage::Done => {
                        let duration = state.start_time.elapsed();
                        completion_duration = duration.as_secs();
                        should_complete = true;
                    }
                    ProgressMessage::Error(err) => {
                        state.progress_messages.push(format!("ERROR: {}", err));
                        // Stay on this screen to show error
                    }
                }
            }
        }

        // Apply state changes after releasing the borrow
        if should_complete {
            self.done_state = Some(DoneState {
                duration_secs: completion_duration,
            });
            self.screen = Screen::Done;
            self.decrypt_state = None;
        }

        if let Some(state) = &self.decrypt_state {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                ui.heading("Decrypting...");
                ui.add_space(40.0);

                // Show file name
                if let Some(name) = state.file_path.file_name().and_then(|n| n.to_str()) {
                    ui.label(format!("File: {}", name));
                }

                ui.add_space(20.0);

                // Show encryption method if detected
                if let Some(method) = &state.encryption_method {
                    ui.label(method);
                    ui.add_space(20.0);
                }

                // Show current section
                if !state.current_section.is_empty() {
                    ui.label(&state.current_section);
                }

                ui.add_space(30.0);

                // Show elapsed time
                let elapsed = state.start_time.elapsed().as_secs();
                ui.label(format!("Elapsed: {}s", elapsed));

                ui.add_space(30.0);

                // Progress area - show recent messages
                ui.group(|ui| {
                    ui.set_min_height(200.0);
                    ui.set_min_width(800.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for msg in state.progress_messages.iter().rev().take(10).rev() {
                                ui.label(msg);
                            }
                        });
                });

                ui.add_space(20.0);
                ui.label("⚠️ Cannot cancel — decryption modifies file in-place");
            });
        }

        ctx.request_repaint();
    }

    fn show_done_screen(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);

            ui.heading("✅ Decryption Complete!");
            ui.add_space(40.0);

            if let Some(done) = &self.done_state {
                ui.label(format!("Total time: {}s", done.duration_secs));
            }

            ui.add_space(60.0);

            let button_size = egui::vec2(400.0, 80.0);

            if ui
                .add_sized(button_size, egui::Button::new("🔄 Decrypt Another"))
                .clicked()
            {
                // Reset to file selection screen
                self.screen = Screen::SelectFile;
                self.selected_file = None;
                self.decrypt_state = None;
                self.done_state = None;
            }

            ui.add_space(20.0);

            if ui
                .add_sized(button_size, egui::Button::new("❌ Quit"))
                .clicked()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        ctx.request_repaint();
    }

    fn load_and_save_keys(&mut self, path: &std::path::Path) {
        match KeyDatabase::from_file(path) {
            Ok(db) => {
                // Save to config directory for auto-detection on next launch
                if let Some(save_path) = KeyDatabase::default_save_path() {
                    if let Err(e) = db.save_to_file(&save_path) {
                        self.key_save_message =
                            Some(format!("⚠️ Keys loaded but could not save: {e}"));
                    } else {
                        self.key_save_message =
                            Some("✅ Keys saved — you won't need to do this again".into());
                    }
                } else {
                    self.key_save_message = Some("✅ Keys loaded".into());
                }

                self.key_status = format!("🔑 Keys loaded ({} keys)", db.len());
                self.keydb = Some(db);
                self.screen = Screen::SelectFile;
            }
            Err(e) => {
                self.key_save_message = Some(format!("❌ Invalid key file: {e}"));
            }
        }
    }

    fn start_decryption(&mut self, path: PathBuf) {
        let (tx, rx): (Sender<ProgressMessage>, Receiver<ProgressMessage>) = channel();

        let decrypt_path = path.clone();
        let keydb = self
            .keydb
            .clone()
            .expect("KeyDatabase must be loaded before decryption");
        thread::spawn(move || {
            let _ = tx.send(ProgressMessage::Started);

            let result =
                citrust_core::decrypt::decrypt_rom(&decrypt_path, &keydb, |progress_text| {
                    let _ = tx.send(ProgressMessage::Update(progress_text.to_string()));
                });

            match result {
                Ok(_) => {
                    let _ = tx.send(ProgressMessage::Done);
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(e.to_string()));
                }
            }
        });

        self.decrypt_state = Some(DecryptState {
            file_path: path,
            progress_messages: Vec::new(),
            current_section: String::new(),
            encryption_method: None,
            start_time: Instant::now(),
            rx,
        });

        self.screen = Screen::Decrypting;
    }

    fn show_key_footer(&mut self, ctx: &egui::Context) {
        let small_style = egui::TextStyle::Name("Small".into());
        egui::TopBottomPanel::bottom("key_footer")
            .exact_height(36.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new(&self.key_status)
                            .text_style(small_style.clone())
                            .color(egui::Color32::from_gray(140)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Browse…").text_style(small_style.clone()),
                                )
                                .frame(false),
                            )
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Key File", &["txt"])
                                .set_title("Select aes_keys.txt")
                                .pick_file()
                        {
                            self.load_and_save_keys(&path);
                        }
                    });
                });
            });
    }
}

impl eframe::App for CitrustApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dark theme for SteamOS aesthetic
        ctx.set_visuals(egui::Visuals::dark());

        // Only show key footer when keys are loaded (not on KeySetup screen)
        if self.keydb.is_some() {
            self.show_key_footer(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.screen {
            Screen::KeySetup => self.show_key_setup_screen(ctx, ui),
            Screen::SelectFile => self.show_select_file_screen(ctx, ui),
            Screen::Decrypting => self.show_decrypting_screen(ctx, ui),
            Screen::Done => self.show_done_screen(ctx, ui),
        });
    }
}
