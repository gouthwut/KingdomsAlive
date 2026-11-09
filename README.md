# Kingdoms Alive

Нативный мод Horizon для автономных деревень. Текущая сборка M0 проверяет, вызывается ли серверный хук при запуске мира.

Для сборки нужен установленный Horizon Nightly `nightly-2026-11-08-fdc87ae`. На Windows запусти из корня проекта, используя путь к установленному лаунчеру:

```powershell
$launcher = "$env:LOCALAPPDATA/Programs/Horizon Launcher/horizon-launcher.exe"
& $launcher instance create 'Kingdoms Alive M0' # Один раз
& $launcher native-build (Resolve-Path mixins/kingdoms_alive).Path --instance kingdoms-alive-m0 --version nightly-2026-11-08-fdc87ae --profile performance --trust-native-code
& $launcher play --instance kingdoms-alive-m0 --world KA-M0-Smoke --creative
```

Для автоматического прогона вместо последней команды запусти `python tools/smoke_m0.py --launcher $launcher`. Скрипт запускает игру через Horizon MCP, ждёт появления игрока и сохраняет подтверждение в `target/m0-smoke/`.

Готовый `.hnmod` можно распаковать и передать путь к папке с `horizon-native.json` в `native-build`. Для упаковки исходников используй `python native/tools/native.py package <путь-к-Kingdoms-Alive>` из репозитория [horizon-mods](https://github.com/horizon-rust/horizon-mods/tree/e2e833d68c3d18ea75af537e73a3bd2bd2bd21fb/native).

Критерий проверки и пути к журналам указаны в [TESTING](docs/TESTING.md). [HANDOFF](docs/HANDOFF.md) содержит текущий статус.
