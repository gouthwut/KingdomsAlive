# Horizon API и совместимость

Цель M0 — Minecraft: Java Edition 26.3 в Horizon Nightly `nightly-2026-11-08-fdc87ae`. Эта ревизия содержит Native Hooks API 411. Установленный лаунчер 0.2.57 собрал план с Horizon Mixins 0.6.0, ревизия `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Точные версии также в [compatibility.json](../compatibility.json).

`.hnmod` содержит Rust-исходники, TOML и метаданные. Лаунчер компилирует их вместе с Horizon для выбранного профиля. В манифесте `hook_version = 1` относится к сигнатуре одного хука, а `hooks = 44` в метаданных пакета — к минимальной версии всего каталога, в которой появился этот хук. [Формат пакета](https://playhorizon.dev/docs/mod-manifest) · [официальный шаблон](https://github.com/horizon-rust/horizon-native-mod-template) · [официальные примеры](https://github.com/horizon-rust/horizon-mods/tree/main/native)

| Возможность | Данные из исходников зафиксированной ревизии | Проверка или ограничение |
| --- | --- | --- |
| Старт и цикл сервера | `server_about_to_start` с `dedicated: bool`, `server_tick`, `level_tick_post`, `server_stopping`. Старт вызывается отдельно интегрированным и выделенным сервером. | M0 проверяет интегрированный запуск. Для авторитетной симуляции M1 проверить частоту тика и порядок lifecycle. |
| Чтение и изменение блоков | `LevelRef` читает состояние и загруженность. `native_set_block` и `native_edit_blocks` меняют мир на сервере. | Проверить отказы для выгруженного чанка и конфликтов. Проверка территории остаётся ответственностью мода. |
| Чанки | `chunk_load` и `chunk_unload` присутствуют в каталоге. | В M6 проверить порядок событий и сверку очереди. Принудительная подгрузка для симуляции не нужна. |
| Сущности | `native_summon`, `native_move_entity`, `native_entity`, чтение UUID/вида, `native_mob_navigate` и `native_mob_navigating`. | Ванильные цели могут перебивать путь. Перед M2 нужно проверить навигацию обычного жителя на живом сервере. [Исходник навигации](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_npcs.rs) |
| Инвентарь | `native_inventory`, `native_move_stack`, `native_set_inventory_slot`, `native_take_stacks`. | Сверить локального игрока и гостя, заполненный инвентарь и повторы. Сквозная атомарность между миром и данными мода не подтверждена. |
| Доска и меню | `native_right_click_block_v2`, `native_open_inventory_menu`; в каталоге есть серверные события взаимодействия. | В M4 проверить активацию существующего блока и доставку событий на клиент. |
| Хранение | `native_world_dir` даёт каталог открытого мира. Доступны `native_world_saved` и хук остановки сервера. | `server_about_to_start` срабатывает до открытия мира. В M1 проверять путь после открытия и на сохранении. Транзакция одного файла не заменяет восстановление после раздельных записей движка и мода. |
| Регистрация деревни и защита | В просмотренном каталоге нет подтверждённого универсального механизма владения постройками игрока. | Начать с явной регистрации одной существующей деревни на выделенной территории. Распознавание деревень и совместимость с защитой участков требуют отдельной проверки. |

Исходники зафиксированной ревизии. [Каталог](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/native-hooks.json) · [Хуки](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_hooks.rs) · [Сервер и путь мира](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_server.rs) · [Сущности](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_entities.rs) · [Инвентарь](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_inventories.rs)

## Автоматизация

Проверено соединение с `horizon-launcher mcp` по stdin/stdout. Сервер 0.2.57 вернул 191 инструмент, включая `play` и `game_hello`. Конфигурация этого компьютера лежит в игнорируемом `.codex/config.toml`. `codex mcp get horizon --json` видит её из корня проекта. Для появления инструментов в чате после изменения настроек открой новый сеанс. [Руководство по управлению](https://playhorizon.dev/docs/agents)
