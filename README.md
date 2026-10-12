# Xash3D-up Engine

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](#) [![Platform](https://img.shields.io/badge/platform-Windows%20x64%20%7C%20Linux%20x64-blue)](#) [![Language](https://img.shields.io/badge/language-C%20%2F%20Rust%20%2F%20Slang-orange)](#) [![Physics](https://img.shields.io/badge/physics-Jolt%20Physics-purple)](#) [![Graphics](https://img.shields.io/badge/graphics-OpenGL%203.3%2B%20%2F%204.6%20PBR-red)](#)

**Xash3D-up** — глубокая модернизация игрового движка Xash3D FWGS, ориентированная на 64-битную архитектуру, интеграцию **Rust**, современный физический движок **Jolt Physics**, графический пайплайн на **Slang** с **PBR (Physically Based Rendering)** и **Realtime освещением**, а также аудиоподсистему на Rust.

---

## Ключевые изменения и возможности

### 🦀 Интеграция Rust
- **Cargo Workspace** в директории `rust/`:
  - [`xash3d-api`](rust/crates/xash3d-api/): кроссплатформенные структуры, FFI-типы и интерфейсы интеграции с движком.
  - [`xash3d-jolt`](rust/crates/xash3d-jolt/): обёртка над высокопроизводительным физическим движком **Jolt Physics** (через Oxijolt / JoltC).
  - [`xash3d-audio`](rust/crates/xash3d-audio/): современный аудиомодуль на Rust с низкими задержками и микшированием.

### ⚡ Jolt Physics
- Замена оригинальной физики твёрдых тел на **Jolt Physics**:
  - Точный расчёт динамики объектов (`MOVETYPE_TOSS`, `MOVETYPE_BOUNCE`, `MOVETYPE_STEP`), импульсов, отскоков и столкновений с полигональным мешем мира.
  - Сохранение 100% совместимости с нативным контроллером движения игрока Half-Life (`pfnPM_Move`), перемещением на вагонетках/лифтах (`func_tracktrain`, `func_plat`), лестницами и CLIP-брашами BSP-карт.

### 🎨 Современный рендерер OpenGL 3.3+ / 4.6 & Slang
- Полностью удалён устаревший Software рендерер и легаси OpenGL-профили. Основной профиль — **OpenGL 3.3+ / 4.6 Compatibility**.
- **Physically Based Rendering (PBR):**
  - Освещение на базе микрогранной модели **Cook-Torrance GGX**.
  - Кинематографичная тональная компрессия **ACES Film Tone Mapping**.
- **Realtime освещение:** динамические точечные источники света в реальном времени (`r_realtime_lights`, включено по умолчанию).
- **Шейдеры на Slang:** исходные шейдеры написаны на языке [Slang](https://shader-slang.com/) в каталоге `shaders/` (`pbr.slang`, `fxaa.slang`, `ssao.slang`).
- **Screen-Space Ambient Occlusion (SSAO):** реалистичное затенение углов и стыков геометрии.
- **FXAA 3.11:** быстрое субпиксельное сглаживание без потери резкости сцены.
- **Автогенерация Normal Maps:** автоматический расчёт рельефных нормалей для текстур по градиентам яркости в экранном пространстве.
- **Интеграция в меню настроек:** переключатели PBR, SSAO и FXAA добавлены в графические настройки игры.

### 🎵 Аудиосистема на Rust
- Аудиоядро вынесено в отдельный Rust-крейт `xash3d-audio` с прямой связкой через C FFI (`engine/client/rust_audio_ffi.h`), обеспечивая чистое воспроизведение и стабильный микшинг.

### 🧹 Очистка кодовой базы
- Полностью вырезана поддержка Android (каталог `android/`, gradle-скрипты, JNI-прослойки и платформозависимые хаки удалены).
- Проект сфокусирован на чистой 64-битной настольной архитектуре (`win32-amd64` / `linux-amd64`).

---

## Настройки графики и консольные переменные (CVars)

| CVar | По умолчанию | Описание |
| :--- | :---: | :--- |
| `r_pbr` | `1` | Включение PBR Cook-Torrance GGX и ACES Tone Mapping |
| `r_realtime_lights` | `1` | Realtime динамическое освещение от источников света |
| `r_ssao` | `1` | Экранное фоновое затенение (SSAO) |
| `r_fxaa` | `1` | Субпиксельное сглаживание FXAA 3.11 |
| `r_lighting_ambient`| `0.2` | Уровень фонового рассеянного освещения сцены |

Все основные эффекты также доступны в меню: **Configuration -> Video -> Advanced**.

---

## Структура репозитория

```
├── engine/              # Ядро движка Xash3D (клиент, сервер, хост, FFI-интерфейсы)
├── ref/gl/              # OpenGL 3.3+ / 4.6 рендерер, интеграция Slang и PBR
├── rust/                # Rust Cargo Workspace
│   └── crates/
│       ├── xash3d-api/  # Общие API типы и FFI интерфейсы
│       ├── xash3d-audio/# Аудиоядро на Rust
│       └── xash3d-jolt/ # Jolt Physics интеграция
├── shaders/             # Slang шейдеры (pbr.slang, fxaa.slang, ssao.slang)
├── game_build/          # Автономная готовая сборка (в .gitignore)
└── game_assets/         # Ресурсы Half-Life и кастомные ассеты (в .gitignore)
```

---

## Сборка из исходного кода

### 1. Требования
- **Компилятор C/C++:** MSVC (Visual Studio 2022) на Windows или GCC/Clang на Linux.
- **Rust:** `rustc` и `cargo` (версия 1.75+).
- **Python 3:** для сборочной системы Waf.
- **SDL2:** библиотека SDL2 (для Windows находится в каталоге `SDL2/`).

### 2. Сборка Rust-компонентов
```bash
cd rust
cargo build --release
cd ..
```
Собранная статическая библиотека `xash3d_jolt.lib` линкуется напрямую с движком.

### 3. Сборка движка (Waf)
```bash
# Конфигурация под 64-бит
python waf configure --sdl2=SDL2 -8

# Сборка движка и рендерера
python waf build
```

Собранные бинарники будут находиться в каталоге `build/`.

---

## Запуск игры

1. Скопируйте папку `valve` из установленной оригинальной **Half-Life** (Steam-версия) в директорию `game_build/`.
2. Запустите `game_build/xash3d.exe`.

---

## Лицензия

Xash3D FWGS распространяется под лицензией **GPL v3**. Подробности в файле `COPYING` или `LICENSE`.
Оригинальный Half-Life является собственностью Valve Corporation.
