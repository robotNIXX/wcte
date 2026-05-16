# Claude Token Counter

Маленький десктоп-виджет под macOS и Windows, который считает токены Claude через официальный эндпоинт `/v1/messages/count_tokens`. Написан на Tauri (Rust + HTML/CSS/JS).

## Что внутри

- **Rust-бэкенд** (`src-tauri/`) — делает запрос к Anthropic API, хранит ключ в системном keychain через крейт `keyring`.
- **Фронт** (`src/`) — vanilla HTML/CSS/JS, без сборщиков. Шрифты грузятся с Google Fonts.
- **Окно** 380×540, ресайзится. В `tauri.conf.json` есть `alwaysOnTop: false` — поменяй на `true`, если хочешь поверх всех окон.

## Требования

- Node.js 18+ (для `@tauri-apps/cli`)
- Rust (`rustup` → стабильный канал)
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: MSVC Build Tools + WebView2 (обычно уже стоит)

Подробности окружения: <https://tauri.app/start/prerequisites/>.

## Запуск

```bash
cd claude-token-counter
npm install
npm run dev      # дев-режим с hot reload фронта
```

Сборка релиза:

```bash
npm run build    # → src-tauri/target/release/bundle/
```

На выходе:
- macOS: `.app` и `.dmg`
- Windows: `.msi` и `.exe`

## Первый запуск

Откроется диалог с просьбой ввести API-ключ Anthropic (получить можно на <https://console.anthropic.com>). Ключ сохраняется в keychain ОС — он не лежит в файлах проекта.

## Управление

- `⌘/Ctrl + Enter` в текстовом поле — посчитать токены
- Шестерёнка справа сверху — настройки ключа
- При смене модели стоимость пересчитывается мгновенно (число токенов то же, цена разная)

## Кастомизация

- **Цены**: в `src/main.js` объект `PRICING` — отредактируй под актуальный прайс <https://www.anthropic.com/pricing>.
- **Модели**: список в `src/index.html` (`<select id="model">`) и пары к нему в `PRICING`.
- **Always-on-top**: `src-tauri/tauri.conf.json` → `windows[0].alwaysOnTop`.
- **Иконки**: лежат в `src-tauri/icons/` — это плейсхолдеры. Замени на свои или сгенерируй через `npx @tauri-apps/cli icon path/to/source.png`.
- **Авто-подсчёт по вводу**: в `src/main.js` можно добавить debounce на `input` event и звать `countNow()` — закомментировал намеренно, чтобы не палить квоту API.

## Структура

```
claude-token-counter/
├── package.json
├── src/
│   ├── index.html
│   ├── main.js
│   └── styles.css
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/
    └── src/
        ├── main.rs
        └── lib.rs
```

## CI

В `.github/workflows/build.yml` есть workflow, который собирает приложение на каждый PR в `main` и при пуше в `main`:

- **macOS** — universal-бинарь (Intel + Apple Silicon), на выходе `.dmg` и `.app`
- **Windows** — x64, на выходе `.msi` и `.exe` (NSIS)

Артефакты складываются в "Artifacts" на странице workflow-run, хранятся 14 дней. Можно скачать без локальной сборки. Rust-кеш через `swatinem/rust-cache` — первая сборка ~10 мин, последующие 2–4. Подписи и нотаризации тут нет — для PR-сборок не нужно.

Запустить руками: вкладка Actions → Build → Run workflow.

## Замечания

- Эндпоинт `count_tokens` бесплатный — Anthropic не списывает деньги за подсчёт.
- Подсчёт — это «estimate», как пишут в доке: фактическое число при реальном вызове Messages API может отличаться на пару токенов из-за системных оптимизаций.
- Картинки и PDF тоже можно считать (эндпоинт это поддерживает), но в этом виджете пока только текст — добавить дроп-зону для файлов несложно.
