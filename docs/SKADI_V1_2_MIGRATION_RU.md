# Миграция с Skadi v1.1 на v1.2

`v1.2` расширяет стабильную инструментальную базу `v1.1`. Обязательной
массовой переписи существующих проектов нет: имя команды остаётся
`skadi-cli`, структура проекта и основные команды совместимы.

## Что проверить после обновления

```bash
skadi-cli --version
skadi-cli doctor
skadi-cli check
skadi-cli format --check
skadi-cli build
```

Версия `skadi-cli` должна быть `1.2.0-rc.1`. Версия проекта в `Skadi.toml`
является версией самого проекта и не обязана совпадать с версией toolchain.

## Новые возможности

- fixed/growing/child/root-static `Memory`, `place in` и явный overflow boundary;
- call-scoped `view`/`edit` и explicit `move` для текущих linear resources;
- native `Task`, `Task(T)`, `run`, `wait`, `stop`, `stopping`;
- bounded `Channel(T)` с blocking `send/receive`;
- nominal `Time`, `Duration`, `ByteSize` и `Angle`;
- `Vec2`, `Vec3`, `Vec4` и ограниченный vector math slice;
- обновлённые CLI/TUI, документация и release installers.

Эти языковые поверхности остаются experimental в линии `v1.2`, даже когда
входят в распространяемый RC toolchain.

## Совместимость математики

`sin` и `cos` по-прежнему принимают числовые radians для совместимости с
`v1.1`, но новый канонический код использует `Angle`:

```skadi
new Angle heading = 30deg
new Float direction_x = cos(heading)
```

## Более строгие проверки concurrency

Результат `run` нельзя игнорировать. Handle задачи должен иметь владельца и
должен быть завершён через `wait`:

```skadi
Task worker_task = run worker()
wait worker_task
```

Для задачи с результатом тип должен совпадать:

```skadi
Task(Int) result_task = run calculate()
new Int result = wait result_task
```

## Установка вместо запуска из исходников

Повседневная документация теперь использует установленную команду:

```bash
skadi-cli check
skadi-cli build
skadi-cli run
```

`cargo run -p skadi-cli -- ...` остаётся поддерживаемым способом разработки
компилятора из checkout репозитория, но не является основным пользовательским
workflow.

## Откат

Установщики заменяют только управляемый бинарник. Чтобы временно вернуться на
предыдущий опубликованный toolchain, повторно запустите установщик с явной
версией. Исходники проектов при этом не изменяются.

Полный список изменений находится в
[`CHANGELOG.md`](https://github.com/eoshipnyagov/Skadi-Language/blob/master/CHANGELOG.md).
