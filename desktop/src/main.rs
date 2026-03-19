#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_file_dialog::FileDialog;
use sgleam_core::{
    engine::Engine,
    error::{show_error, SgleamError},
    gleam::{compile, Project},
    quickjs::{capture_output, QuickJsEngine},
};
use std::path::PathBuf;

enum PendingDialog {
    None,
    Open,
    Save,
}

struct App {
    code: String,
    output: String,
    file_path: Option<PathBuf>,
    file_dialog: FileDialog,
    pending: PendingDialog,
    split_ratio: f32,
    dragging: bool,
}

impl App {
    fn new() -> Self {
        App {
            code: String::from(
                "import sgleam/check\n\
                 \n\
                 pub fn hello(name: String) -> String {\n\
                 \x20\x20\"Hello \" <> name <> \"!\"\n\
                 }\n\
                 \n\
                 pub fn hello_examples() {\n\
                 \x20\x20check.eq(hello(\"World\"), \"Hello World!\")\n\
                 }\n",
            ),
            output: String::new(),
            file_path: None,
            file_dialog: FileDialog::new(),
            pending: PendingDialog::None,
            split_ratio: 0.5,
            dragging: false,
        }
    }

    fn run_code(&mut self) {
        let (out, err) = capture_output(|| {
            // Compile and run tests (same as web version)
            let mut project = Project::default();
            project.write_source("code.gleam", &self.code);
            let modules = match compile(&mut project, true) {
                Err(err) => {
                    show_error(&SgleamError::Gleam(err));
                    return;
                }
                Ok(modules) => modules,
            };
            let has_examples = modules.iter().any(|m| {
                m.name == "code"
                    && m.ast.definitions.functions.iter().any(|f| {
                        f.publicity.is_public()
                            && f.name
                                .as_ref()
                                .map(|n| n.1.ends_with("_examples"))
                                .unwrap_or(false)
                    })
            });
            if has_examples {
                QuickJsEngine::new(project.fs.clone()).run_tests(&["code"]);
            }
        });

        self.output = String::new();
        if !err.is_empty() {
            self.output.push_str(&err);
        }
        if !out.is_empty() {
            self.output.push_str(&out);
        }
        if self.output.is_empty() {
            self.output = String::from("(no output)");
        }
    }

    fn open_file(&mut self) {
        self.pending = PendingDialog::Open;
        self.file_dialog.pick_file();
    }

    fn save_file(&mut self) {
        if let Some(ref path) = self.file_path {
            let _ = std::fs::write(path, &self.code);
        } else {
            self.pending = PendingDialog::Save;
            self.file_dialog.save_file();
        }
    }

    fn handle_dialog(&mut self) {
        if let Some(path) = self.file_dialog.take_picked() {
            match self.pending {
                PendingDialog::Open => {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        self.code = content;
                        self.file_path = Some(path);
                    }
                }
                PendingDialog::Save => {
                    let _ = std::fs::write(&path, &self.code);
                    self.file_path = Some(path);
                }
                PendingDialog::None => {}
            }
            self.pending = PendingDialog::None;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
            self.run_code();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.save_file();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.open_file();
        }

        // File dialog
        self.file_dialog.update(ctx);
        self.handle_dialog();

        // Top panel with buttons
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("▶ Run (Ctrl+R)").clicked() {
                    self.run_code();
                }
                if ui.button("📂 Open (Ctrl+O)").clicked() {
                    self.open_file();
                }
                if ui.button("💾 Save (Ctrl+S)").clicked() {
                    self.save_file();
                }
                if let Some(ref path) = self.file_path {
                    ui.label(
                        egui::RichText::new(path.display().to_string())
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                }
            });
        });

        // Main area with manual split
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let total_width = rect.width();
            let separator_width = 8.0;
            let editor_width = (total_width * self.split_ratio - separator_width / 2.0).max(50.0);
            let output_x = rect.left() + editor_width + separator_width;
            let output_width = (total_width - editor_width - separator_width).max(50.0);

            // Editor area (left)
            let editor_rect =
                egui::Rect::from_min_size(rect.left_top(), egui::vec2(editor_width, rect.height()));
            let mut editor_ui = ui.new_child(egui::UiBuilder::new().max_rect(editor_rect));
            editor_ui.heading("Editor");
            let editor_avail = editor_ui.available_size();
            egui::ScrollArea::both().show(&mut editor_ui, |ui| {
                ui.add_sized(
                    editor_avail,
                    egui::TextEdit::multiline(&mut self.code)
                        .font(egui::TextStyle::Monospace)
                        .code_editor(),
                );
            });

            // Separator (draggable)
            let sep_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + editor_width, rect.top()),
                egui::vec2(separator_width, rect.height()),
            );
            let sep_response = ui.allocate_rect(sep_rect, egui::Sense::drag());
            ui.painter().rect_filled(
                sep_rect,
                0.0,
                if sep_response.hovered() || self.dragging {
                    ui.visuals().widgets.active.bg_fill
                } else {
                    ui.visuals().widgets.noninteractive.bg_fill
                },
            );

            if sep_response.drag_started() {
                self.dragging = true;
            }
            if self.dragging {
                if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                    let new_ratio = (pos.x - rect.left()) / total_width;
                    self.split_ratio = new_ratio.clamp(0.1, 0.9);
                }
                ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
            }
            if ctx.input(|i| !i.pointer.primary_down()) {
                self.dragging = false;
            }
            if sep_response.hovered() {
                ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
            }

            // Output area (right)
            let output_rect = egui::Rect::from_min_size(
                egui::pos2(output_x, rect.top()),
                egui::vec2(output_width, rect.height()),
            );
            let mut output_ui = ui.new_child(egui::UiBuilder::new().max_rect(output_rect));
            output_ui.heading("Output");
            let output_avail = output_ui.available_size();
            egui::ScrollArea::both().show(&mut output_ui, |ui| {
                ui.add_sized(
                    output_avail,
                    egui::TextEdit::multiline(&mut self.output.as_str())
                        .font(egui::TextStyle::Monospace),
                );
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 600.0])
            .with_title("Sgleam"),
        ..Default::default()
    };
    eframe::run_native(
        "Sgleam",
        options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            // Set larger default font sizes
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                egui::FontId::new(16.0, egui::FontFamily::Monospace),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            ctx.set_style(style);
            Ok(Box::new(App::new()))
        }),
    )
}
