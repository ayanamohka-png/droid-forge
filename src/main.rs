mod android;

use eframe::egui;

#[derive(PartialEq)]
enum Tab {
    Adb,
    Fastboot,
}

struct DroidForgeApp {
    current_tab: Tab,
    adb_connected: bool,
    fastboot_connected: bool,
    log_text: String,

    // ADB состояние
    selected_apk: Option<String>,
    selected_sideload: Option<String>,
    selected_push_file: Option<String>,
    pull_remote_path: String,

    // Fastboot состояние
    selected_partition: String,
    selected_image: Option<String>,
    partitions: Vec<String>,
    fastboot_var: String,
}

impl Default for DroidForgeApp {
    fn default() -> Self {
        Self {
            current_tab: Tab::Adb,
            adb_connected: false,
            fastboot_connected: false,
            log_text: String::from("[INFO] Добро пожаловать в DroidForge!\n[INFO] Нажмите 'Обновить статус' для проверки подключения\n"),

            selected_apk: None,
            selected_sideload: None,
            selected_push_file: None,
            pull_remote_path: String::from("/sdcard/"),

            selected_partition: "boot".to_string(),
            selected_image: None,
            partitions: vec![
                "boot".to_string(),
                "recovery".to_string(),
                "system".to_string(),
                "vendor".to_string(),
                "radio".to_string(),
            ],
            fastboot_var: "product".to_string(),
        }
    }
}

impl DroidForgeApp {
    fn add_log(&mut self, msg: &str) {
        self.log_text.push_str(msg);
        self.log_text.push('\n');
    }

    fn check_connection_status(&mut self) {
        self.add_log("[INFO] Проверка подключения...");

        let (adb, _adb_log) = android::check_adb();
        self.adb_connected = adb;
        if adb {
            self.add_log("[ADB] ✓ Устройство подключено в режиме ADB");
        } else {
            self.add_log("[ADB] ✗ Устройство не найдено");
        }

        let (fb, fb_log) = android::check_fastboot();
        self.fastboot_connected = fb;
        if fb {
            self.add_log("[FASTBOOT] ✓ Устройство подключено в режиме Fastboot");
        } else {
            self.add_log("[FASTBOOT] ✗ Устройство не найдено");
        }
        if !fb_log.trim().is_empty() {
            self.add_log(&format!("  {}", fb_log.trim().replace('\n', "\n  ")));
        }
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let adb_color = if self.adb_connected {
                egui::Color32::from_rgb(34, 211, 238) // cyan
            } else {
                egui::Color32::from_rgb(100, 100, 100)
            };

            let fb_color = if self.fastboot_connected {
                egui::Color32::from_rgb(34, 211, 238)
            } else {
                egui::Color32::from_rgb(100, 100, 100)
            };

            ui.colored_label(adb_color, if self.adb_connected { "● ADB: Подключен" } else { "○ ADB: Отключен" });
            ui.separator();
            ui.colored_label(fb_color, if self.fastboot_connected { "● Fastboot: Подключен" } else { "○ Fastboot: Отключен" });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Обновить статус").clicked() {
                    self.check_connection_status();
                }
            });
        });
    }

    fn render_adb_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Установка и файлы
            ui.group(|ui| {
                ui.heading("📦 Установка и файлы");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать APK").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("APK files", &["apk"])
                            .pick_file()
                        {
                            self.selected_apk = Some(path.display().to_string());
                            self.add_log(&format!("[INFO] Выбран APK: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_apk.is_some(), |ui| {
                        if ui.button("⚡ Установить APK").clicked() {
                            if let Some(apk_path) = self.selected_apk.clone() {
                                self.add_log(&format!("[ADB] Установка APK: {}", apk_path));
                                match android::install_apk(&apk_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(apk) = &self.selected_apk {
                        ui.label(apk);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать ZIP").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("ZIP files", &["zip"])
                            .pick_file()
                        {
                            self.selected_sideload = Some(path.display().to_string());
                            self.add_log(&format!("[INFO] Выбран ZIP: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_sideload.is_some(), |ui| {
                        if ui.button("⚡ Sideload (Recovery)").clicked() {
                            if let Some(zip_path) = self.selected_sideload.clone() {
                                self.add_log("⚠ Убедитесь, что устройство в Recovery!");
                                self.add_log(&format!("[ADB] Sideload: {}", zip_path));
                                match android::sideload(&zip_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(zip) = &self.selected_sideload {
                        ui.label(zip);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Выбрать файл").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.selected_push_file = Some(path.display().to_string());
                            self.add_log(&format!("[INFO] Выбран файл: {}", path.display()));
                        }
                    }

                    ui.add_enabled_ui(self.adb_connected && self.selected_push_file.is_some(), |ui| {
                        if ui.button("⬆ Передать файл на /sdcard/").clicked() {
                            if let Some(file_path) = &self.selected_push_file.clone() {
                                self.add_log(&format!("[ADB] Push: {}", file_path));
                                match android::push_file(file_path, "/sdcard/") {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }
                    });

                    if let Some(file) = &self.selected_push_file {
                        ui.label(file);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Путь на устройстве:");
                    ui.text_edit_singleline(&mut self.pull_remote_path);

                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("⬇ Забрать файл").clicked() {
                            if let Some(path) = rfd::FileDialog::new().save_file() {
                                let local_path = path.display().to_string();
                                self.add_log(&format!("[ADB] Pull: {} -> {}", self.pull_remote_path, local_path));
                                match android::pull_file(&self.pull_remote_path, &local_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Ребут и режимы
            ui.group(|ui| {
                ui.heading("🔄 Ребут и режимы");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("В систему").clicked() {
                            self.add_log("[ADB] Перезагрузка в систему...");
                            match android::adb_reboot("") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }

                        if ui.button("В Recovery").clicked() {
                            self.add_log("[ADB] Перезагрузка в Recovery...");
                            match android::adb_reboot("recovery") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }

                        if ui.button("В Fastboot").clicked() {
                            self.add_log("[ADB] Перезагрузка в Fastboot...");
                            match android::adb_reboot("bootloader") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Информация и отладка
            ui.group(|ui| {
                ui.heading("🔍 Информация и отладка");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.adb_connected, |ui| {
                        if ui.button("📸 Скриншот").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("screenshot.png")
                                .add_filter("PNG", &["png"])
                                .save_file()
                            {
                                let output_path = path.display().to_string();
                                self.add_log("[ADB] Создание скриншота...");
                                match android::screenshot(&output_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] Сохранено в {}\n{}", output_path, log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }

                        if ui.button("📋 Логи (Logcat)").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("logcat.txt")
                                .add_filter("TXT", &["txt"])
                                .save_file()
                            {
                                let output_path = path.display().to_string();
                                self.add_log("[ADB] Сбор логов...");
                                match android::logcat(&output_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }

                        if ui.button("ℹ Инфо о телефоне").clicked() {
                            self.add_log("[ADB] Получение информации...");
                            match android::get_device_info() {
                                Ok(info) => self.add_log(&format!("[INFO]\n{}", info)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });
            });
        });
    }

    fn render_fastboot_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Прошивка
            ui.group(|ui| {
                ui.heading("⚡ Прошивка");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Раздел:");
                    egui::ComboBox::from_id_salt("partition_select")
                        .selected_text(&self.selected_partition)
                        .show_ui(ui, |ui| {
                            for partition in &self.partitions {
                                ui.selectable_value(
                                    &mut self.selected_partition,
                                    partition.clone(),
                                    partition
                                );
                            }
                        });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("📁 Выбрать .img").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Image files", &["img"])
                            .pick_file()
                        {
                            self.selected_image = Some(path.display().to_string());
                            self.add_log(&format!("[INFO] Выбран образ: {}", path.display()));
                        }
                    }

                    if let Some(img) = &self.selected_image {
                        ui.label(img);
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected && self.selected_image.is_some(), |ui| {
                        if ui.button("⚡ Прошить").clicked() {
                            if let Some(img_path) = self.selected_image.clone() {
                                let partition = self.selected_partition.clone();
                                self.add_log(&format!("[FASTBOOT] Прошивка {} <- {}", partition, img_path));
                                match android::flash_partition(&partition, &img_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }

                        if ui.button("🚀 Загрузиться с образа").clicked() {
                            if let Some(img_path) = self.selected_image.clone() {
                                self.add_log(&format!("[FASTBOOT] Boot (без прошивки): {}", img_path));
                                match android::boot_image(&img_path) {
                                    Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                    Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Управление загрузчиком
            ui.group(|ui| {
                ui.heading("🔓 Управление загрузчиком");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("🔓 Разблокировать").clicked() {
                            self.add_log("⚠ ВНИМАНИЕ: Разблокировка СОТРЕТ ВСЕ ДАННЫЕ!");
                            self.add_log("[FASTBOOT] Разблокировка загрузчика...");
                            match android::unlock_bootloader() {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }

                        if ui.button("🔒 Заблокировать").clicked() {
                            self.add_log("[FASTBOOT] Блокировка загрузчика...");
                            match android::lock_bootloader() {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Стереть раздел:");
                    egui::ComboBox::from_id_salt("erase_partition")
                        .selected_text(&self.selected_partition)
                        .show_ui(ui, |ui| {
                            for partition in &self.partitions {
                                ui.selectable_value(
                                    &mut self.selected_partition,
                                    partition.clone(),
                                    partition
                                );
                            }
                        });

                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("🗑 Стереть").clicked() {
                            self.add_log(&format!("⚠ Стирание раздела {}...", self.selected_partition));
                            match android::erase_partition(&self.selected_partition) {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Ребут
            ui.group(|ui| {
                ui.heading("🔄 Ребут");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("Перезагрузить").clicked() {
                            self.add_log("[FASTBOOT] Перезагрузка...");
                            match android::fastboot_reboot("") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }

                        if ui.button("В Recovery").clicked() {
                            self.add_log("[FASTBOOT] Перезагрузка в Recovery...");
                            match android::fastboot_reboot("recovery") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }

                        if ui.button("В Bootloader").clicked() {
                            self.add_log("[FASTBOOT] Перезагрузка в Bootloader...");
                            match android::fastboot_reboot("bootloader") {
                                Ok(log) => self.add_log(&format!("[SUCCESS] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);

            // Fastboot getvar
            ui.group(|ui| {
                ui.heading("ℹ Информация");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Переменная:");
                    ui.text_edit_singleline(&mut self.fastboot_var);

                    ui.add_enabled_ui(self.fastboot_connected, |ui| {
                        if ui.button("📋 getvar").clicked() {
                            self.add_log(&format!("[FASTBOOT] getvar {}...", self.fastboot_var));
                            match android::get_fastboot_var(&self.fastboot_var) {
                                Ok(log) => self.add_log(&format!("[INFO] {}", log)),
                                Err(e) => self.add_log(&format!("[ERROR] {}", e)),
                            }
                        }
                    });
                });

                ui.add_space(4.0);
                ui.label("Примеры: product, version-bootloader, version-baseband, serialno");
            });
        });
    }
}

impl eframe::App for DroidForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // Верхняя панель со статусом
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            self.render_status_bar(ui);
            ui.add_space(4.0);
        });

        // Нижняя панель с логом
        egui::TopBottomPanel::bottom("log_panel").min_height(200.0).show(ctx, |ui| {
            ui.heading("📋 Лог");
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.log_text)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(12)
                    );
                });
        });

        // Центральная панель с вкладками
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            // Вкладки
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Adb, "📱 ADB");
                ui.selectable_value(&mut self.current_tab, Tab::Fastboot, "⚡ Fastboot");
            });

            ui.separator();
            ui.add_space(8.0);

            match self.current_tab {
                Tab::Adb => self.render_adb_tab(ui),
                Tab::Fastboot => self.render_fastboot_tab(ui),
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_title("DroidForge"),
        ..Default::default()
    };

    eframe::run_native(
        "DroidForge",
        options,
        Box::new(|_cc| Ok(Box::new(DroidForgeApp::default()))),
    )
}
