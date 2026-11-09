# Передача контекста

## Сейчас

M1 завершён. Сборка `native-fdc87ae-94f338c62618` в профиле `kingdoms-alive-m1` зарегистрировала деревню в `KA-M1-Village`. После перезапуска совпали ID, UUID четырёх жителей и склад, приоритет вычислился заново. Повреждённый снимок сохранён в ходе проверки, в мире восстановлен исходный. Подтверждения в `target/m1-smoke/`, подробности в [PROGRESS](PROGRESS.md).

## GitHub

Публичный [gouthwut/KingdomsAlive](https://github.com/gouthwut/KingdomsAlive) под MIT © 2026 gouthwut. `origin/main` на `b99df99`, [CI прошёл](https://github.com/gouthwut/KingdomsAlive/actions/runs/42598110224). M1 в локальном коммите `main`. Теги, GitHub Releases и отправка `.hnmod` требуют отдельного разрешения. [Порядок публикации](RELEASING.md).

## Следующие действия

Следующий этап M2. Сначала проверь, что `native_mob_navigate` ведёт обычного жителя до цели в тестовом мире. Затем добавь назначение работы и первый полный цикл добычи и доставки по [ROADMAP](ROADMAP.md).

## Где смотреть

- `mixins/kingdoms_alive/core/` — модель и снимок. `cargo test --locked` из корня.
- `mixins/kingdoms_alive/settlement_server.rs` — серверный адаптер.
- [TESTING](TESTING.md) — повтор игрового прогона. [HORIZON_API](HORIZON_API.md) — закреплённая ревизия и проверенные вызовы.
