<div align="center">

# ⚡ DroidForge 🔧

**Легкий и быстрый графический интерфейс для ADB и Fastboot.**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20windows%20%7C%20macos-lightgrey?style=for-the-badge)]()

Забудьте о командной строке. Устанавливайте APK, делайте скриншоты, прошивайте рекавери и разблокируйте загрузчик в пару кликов.

[Скачать последнюю версию](https://github.com/ayanamohka-png/droidforge/releases) • [Сообщить об ошибке](https://github.com/ayanamohka-png/droidforge/issues)

</div>

---

## 📸 Скриншоты

| Вкладка ADB (Установка и файлы) | Вкладка Fastboot (Прошивка) |
|:---:|:---:|
| ![ADB](assets/screenshot_adb.png) | ![Fastboot](assets/screenshot_fastboot.png) |

---

## ✨ Что умеет DroidForge?

**В режиме ADB (телефон включен):**
* 📦 Установка `.apk` файлов.
* 📸 Создание скриншотов экрана.
* 📂 Передача файлов на телефон (`adb push`) и обратно (`adb pull`).
* 🔄 Sideload ZIP-архивов (для TWRP).
* 📊 Просмотр информации о телефоне и логов.

**В режиме Fastboot (режим загрузчика):**
* 🛠 Прошивка разделов `boot`, `recovery`, `system`.
* 🔓 Разблокировка и блокировка загрузчика (`flashing unlock`).
* 🧹 Стирание разделов (`erase`).
* 🔄 Перезагрузка в систему, рекавери или фастбут.

---

## 🚀 Как установить

Для работы программы на вашем компьютере должны быть установлены утилиты **ADB** и **Fastboot** (Android Platform-Tools).

### 🐧 Linux (Arch, Ubuntu, Debian, Fedora)
1. Установите зависимости:
   * **Arch / CachyOS:** `sudo pacman -S android-tools`
   * **Ubuntu / Debian / Mint:** `sudo apt install android-tools-adb android-tools-fastboot`
   * **Fedora:** `sudo dnf install android-tools`
2. Скачайте **`.AppImage`** файл из раздела [Releases](https://github.com/ТВОЙ_НИК/droidforge/releases).
3. Сделайте его исполняемым: `chmod +x DroidForge*.AppImage`
4. Запустите: `./DroidForge*.AppImage`

### 🪟 Windows
1. Скачайте **[SDK Platform-Tools](https://developer.android.com/studio/releases/platform-tools)** с официального сайта Google и распакуйте архив.
2. Добавьте папку `platform-tools` в переменную среды `PATH` (или просто положите `DroidForge.exe` в эту папку).
3. Скачайте **`DroidForge.exe`** из раздела [Releases](https://github.com/ТВОЙ_НИК/droidforge/releases).
4. Запустите.

### 🍏 macOS
*Сборка для macOS появится в ближайших обновлениях.*

---

## 🛠 Сборка из исходников (для разработчиков)

```bash
# 1. Клонируем репозиторий
git clone https://github.com/ТВОЙ_НИК/droidforge.git
cd droidforge

# 2. Собираем релиз
cargo build --release

# 3. Запускаем
./target/release/droidforge
