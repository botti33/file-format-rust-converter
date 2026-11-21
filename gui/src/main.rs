use std::collections::BTreeMap;
use std::path::PathBuf;

use anyconvert_core::{
    all_conversions, convert_file, extension_of, format, targets_for, ConvertOptions,
};
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Convert,
    Formats,
}

struct LogEntry {
    text: String,
    ok: bool,
}

struct AppState {
    files: Vec<PathBuf>,
    target: String,
    out_dir: Option<PathBuf>,
    quality: u8,
    overwrite: bool,
    tab: Tab,
    log: Vec<LogEntry>,
    capabilities: Vec<(&'static str, Vec<&'static str>)>,
}

impl AppState {
    fn new() -> Self {
        let mut groups: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
        for conversion in all_conversions() {
            groups
                .entry(conversion.from)
                .or_default()
                .push(conversion.to);
        }
        let capabilities = groups
            .into_iter()
            .map(|(from, mut targets)| {
                targets.sort_unstable();
                targets.dedup();
                (from, targets)
            })
            .collect();

        Self {
            files: Vec::new(),
            target: "webp".to_string(),
            out_dir: None,
            quality: 85,
            overwrite: false,
            tab: Tab::Convert,
            log: Vec::new(),
            capabilities,
        }
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    let children: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
                    self.add_paths(children);
                }
            } else if path.is_file() && !self.files.contains(&path) {
                self.files.push(path);
            }
        }
        self.refresh_target();
    }

    fn available_targets(&self) -> Vec<String> {
        if self.files.is_empty() {
            return vec![
                "png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "qoi", "json", "yaml",
                "toml", "csv", "xml", "html", "md", "txt", "pdf", "wav",
            ]
            .into_iter()
            .map(str::to_string)
            .collect();
        }

        let mut common: Option<Vec<&'static str>> = None;
        for file in &self.files {
            let ext = extension_of(file).unwrap_or_default();
            let targets = targets_for(&ext);
            common = Some(match common {
                None => targets,
                Some(existing) => existing
                    .into_iter()
                    .filter(|t| targets.contains(t))
                    .collect(),
            });
        }
        let mut targets = common.unwrap_or_default();
        if targets.is_empty() {
            let ext = extension_of(&self.files[0]).unwrap_or_default();
            targets = targets_for(&ext);
        }
        targets.into_iter().map(str::to_string).collect()
    }

    fn refresh_target(&mut self) {
        let options = self.available_targets();
        if !options.iter().any(|t| t == &self.target) {
            self.target = suggest_target(&self.files.first().and_then(|p| extension_of(p)).unwrap_or_default())
                .filter(|s| options.iter().any(|t| t == s))
                .or_else(|| options.first().cloned())
                .unwrap_or_else(|| self.target.clone());
        }
    }

    fn convert(&mut self) {
        self.log.clear();
        let options = ConvertOptions {
            quality: self.quality.clamp(1, 100),
            from_ext: None,
        };
        let mut ok = 0usize;
        let mut failed = 0usize;

        for input in &self.files {
            let Some(from) = extension_of(input) else {
                self.log.push(LogEntry {
                    text: format!("{}: cannot detect source type", input.display()),
                    ok: false,
                });
                failed += 1;
                continue;
            };
            if !targets_for(&from).contains(&self.target.as_str()) {
                self.log.push(LogEntry {
                    text: format!("{}: .{from} -> .{} not supported", input.display(), self.target),
                    ok: false,
                });
                failed += 1;
                continue;
            }

            let name = input.with_extension(&self.target);
            let output = match &self.out_dir {
                Some(dir) => dir.join(name.file_name().unwrap_or_default()),
                None => name,
            };
            if output.exists() && !self.overwrite {
                self.log.push(LogEntry {
                    text: format!("{}: output exists (enable overwrite)", output.display()),
                    ok: false,
                });
                failed += 1;
                continue;
            }

            match convert_file(input, &output, &self.target, &options) {
                Ok(outcome) => {
                    ok += 1;
                    self.log.push(LogEntry {
                        text: format!("{} -> {}  ({} bytes)", input.display(), outcome.output.display(), outcome.bytes),
                        ok: true,
                    });
                }
                Err(err) => {
                    failed += 1;
                    self.log.push(LogEntry {
                        text: format!("{}: {err}", input.display()),
                        ok: false,
                    });
                }
            }
        }

        self.log.push(LogEntry {
            text: format!("Finished: {ok} converted, {failed} failed"),
            ok: failed == 0,
        });
    }
}

fn suggest_target(ext: &str) -> Option<String> {
    if ext.is_empty() {
        return None;
    }
    let target = match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "ico" | "qoi" => "webp",
        "json" | "yaml" | "toml" | "csv" | "xml" | "ron" | "json5" | "ini" => "json",
        "md" => "html",
        "html" => "md",
        "pdf" => "txt",
        "mp3" | "flac" | "ogg" | "aac" | "m4a" | "opus" | "aiff" => "wav",
        _ => "txt",
    };
    format(target).map(|f| f.ext.to_string())
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([940.0, 660.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("AnyConvert"),
        ..Default::default()
    };

    let mut state = AppState::new();

    eframe::run_ui_native("AnyConvert", options, move |ui, _frame| {
        // Handle files dropped anywhere in the window.
        let dropped: Vec<PathBuf> = ui
            .input(|i| i.raw.dropped_files.clone())
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect();
        if !dropped.is_empty() {
            state.add_paths(dropped);
        }

        egui::Frame::new()
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("AnyConvert");
                    ui.label(egui::RichText::new("offline • fast • local").weak().small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(state.tab == Tab::Formats, "Supported formats").clicked() {
                            state.tab = Tab::Formats;
                        }
                        if ui.selectable_label(state.tab == Tab::Convert, "Convert").clicked() {
                            state.tab = Tab::Convert;
                        }
                    });
                });
                ui.separator();

                match state.tab {
                    Tab::Convert => convert_ui(ui, &mut state),
                    Tab::Formats => formats_ui(ui, &state),
                }
            });
    })
}

fn convert_ui(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("Add files…").clicked() {
            if let Some(files) = rfd::FileDialog::new().pick_files() {
                state.add_paths(files);
            }
        }
        if ui.button("Add folder…").clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.add_paths(vec![folder]);
            }
        }
        if ui.button("Clear").clicked() {
            state.files.clear();
            state.log.clear();
            state.target = "webp".to_string();
        }
        ui.label(egui::RichText::new("or drag & drop files / folders here").weak());
    });

    ui.add_space(6.0);

    egui::Frame::new()
        .inner_margin(egui::Margin::same(10))
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(6.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("file-list")
                .max_height(230.0)
                .show(ui, |ui| {
                    if state.files.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("No files yet — add some to get started")
                                    .weak()
                                    .size(16.0),
                            );
                        });
                    } else {
                        let mut remove = None;
                        for (index, path) in state.files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let ext = extension_of(path).unwrap_or_else(|| "?".to_string());
                                ui.label(
                                    egui::RichText::new(format!(".{ext}"))
                                        .color(egui::Color32::from_rgb(120, 190, 255))
                                        .monospace(),
                                );
                                ui.label(path.to_string_lossy());
                                if ui.small_button("✕").clicked() {
                                    remove = Some(index);
                                }
                            });
                        }
                        if let Some(index) = remove {
                            state.files.remove(index);
                        }
                    }
                });
        });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label("Convert to:");
        let options = state.available_targets();
        egui::ComboBox::from_id_salt("target-format")
            .selected_text(format!(".{}", state.target))
            .show_ui(ui, |ui| {
                for target in &options {
                    ui.selectable_value(&mut state.target, target.clone(), format!(".{target}"));
                }
            });

        if state.target == "jpg" || state.target == "webp" {
            ui.label("Quality");
            ui.add(egui::Slider::new(&mut state.quality, 1..=100));
        }
        ui.checkbox(&mut state.overwrite, "Overwrite existing");
    });

    ui.horizontal(|ui| {
        ui.label("Output folder:");
        match &state.out_dir {
            Some(dir) => ui.label(dir.to_string_lossy()),
            None => ui.label(egui::RichText::new("same as source").weak()),
        };
        if ui.button("Choose…").clicked() {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                state.out_dir = Some(dir);
            }
        }
        if state.out_dir.is_some() && ui.button("Reset").clicked() {
            state.out_dir = None;
        }
    });

    ui.add_space(10.0);
    let can_convert = !state.files.is_empty() && !state.target.is_empty();
    if ui
        .add_enabled(can_convert, egui::Button::new(egui::RichText::new("Convert").size(16.0).strong()))
        .clicked()
    {
        state.convert();
    }

    if !state.log.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("log")
            .max_height(160.0)
            .show(ui, |ui| {
                for entry in &state.log {
                    let color = if entry.ok {
                        egui::Color32::from_rgb(130, 220, 160)
                    } else {
                        egui::Color32::from_rgb(255, 140, 140)
                    };
                    ui.label(egui::RichText::new(&entry.text).color(color).monospace().small());
                }
            });
    }
}

fn formats_ui(ui: &mut egui::Ui, state: &AppState) {
    ui.label(
        egui::RichText::new(format!(
            "{} supported conversions — all performed locally, no uploads.",
            all_conversions().len()
        )),
    );
    ui.add_space(6.0);

    egui::ScrollArea::vertical().id_salt("capabilities").show(ui, |ui| {
        for (from, targets) in &state.capabilities {
            ui.collapsing(format!(".{from}  →  {} formats", targets.len()), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for target in targets {
                        ui.label(
                            egui::RichText::new(format!(".{target}"))
                                .monospace()
                                .color(egui::Color32::from_rgb(150, 200, 255)),
                        );
                    }
                });
            });
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label(
        egui::RichText::new(
            "Notes: animated images are flattened to their first frame. Audio decodes to 16-bit PCM WAV. Generated PDFs contain basic text or an embedded JPEG.",
        )
        .weak()
        .small(),
    );
}
