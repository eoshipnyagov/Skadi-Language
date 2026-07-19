# Showcase-программы Skadi

В этом разделе собраны 13 небольших showcase-программ.

Их цель:

- проверять, что разные реальные формы программ проходят через текущий путь
  `Skadi -> C -> executable`;
- служить витриной текущего языка;
- давать быстрый smoke-набор после изменений в компиляторе.

## Программы

<ol>
  <li><code>bench_01_tree.skd</code><ul><li>Рекурсивный обход директорий.</li><li>Покрытие: рекурсия, <code>fs.list</code>, <code>fs.join</code>, <code>fs.is_dir</code>, разбор флагов через <code>when</code>.</li></ul></li>
  <li><code>bench_02_read_stats.skd</code><ul><li>Читает файл и печатает статистику по символам и строкам.</li><li>Покрытие: файловый ввод (<code>read</code>), циклы по тексту, <code>slice</code> и <code>find</code>.</li></ul></li>
  <li><code>bench_03_find_count.skd</code><ul><li>Считает количество вхождений подстроки в содержимом файла.</li><li>Покрытие: сканирование текста, управляющие конструкции, композиция builtins.</li></ul></li>
  <li><code>bench_04_sum_ints.skd</code><ul><li>Собирает список целых чисел и считает сумму.</li><li>Покрытие: списки, <code>push</code>, итерация по списку, арифметика.</li></ul></li>
  <li><code>bench_05_push_pop.skd</code><ul><li>Работает со списком как со стеком.</li><li>Покрытие: изменение списка, <code>pop() on error</code>, управление циклом.</li></ul></li>
  <li><code>bench_06_struct_account.skd</code><ul><li>Минимальная модель счёта с методами.</li><li>Покрытие: <code>struct</code>, <code>my.field</code>, вызовы методов (<code>obj.method(...)</code>), типизированный struct literal.</li></ul></li>
  <li><code>bench_07_struct_list.skd</code><ul><li>Обход списка структур с проверками через методы.</li><li>Покрытие: <code>Struct List</code>, <code>push</code> структурных литералов, <code>iterate ... as ...</code>, вызовы методов на элементе списка.</li></ul></li>
  <li><code>bench_08_path_list_helpers.skd</code><ul><li>Утилита для обхода и фильтрации путей.</li><li>Покрытие: <code>Path List</code>, <code>fs.list</code>, <code>fs.join</code>, <code>fs.is_dir</code>, <code>iterate ... as ...</code>.</li></ul></li>
  <li><code>bench_09_math_navigation.skd</code><ul><li>Небольшой showcase для math и навигации.</li><li>Покрытие: <code>deg_to_rad</code>, <code>sin</code>, <code>cos</code>, <code>atan2</code>, <code>sqrt</code>, <code>round</code>, <code>clamp</code>.</li></ul></li>
  <li><code>bench_10_v1_1_toolbox.skd</code><ul><li>Сборный showcase для ключевых обновлений <code>v1.1</code>.</li><li>Покрытие: <code>danger fn</code>, <code>on error</code>, <code>label ErrorCode</code>, <code>struct</code>, методы, <code>List</code>, <code>iterate ... as ...</code>, <code>when</code>, math core и cleanup generated C.</li></ul></li>
  <li><code>bench_11_task_channel_pipeline.skd</code><ul><li>Исполняемый showcase concurrency slice <code>v1.2</code>.</li><li>Покрытие: <code>Task</code>, <code>Task(Float)</code>, bounded <code>Channel(Reading)</code>, capacity-1 backpressure, struct messages, blocking <code>send/receive</code> и обязательный <code>wait</code>.</li></ul></li>
  <li><code>bench_12_systems_pipeline.skd</code><ul><li>Совместный systems showcase <code>v1.2</code>.</li><li>Покрытие: fixed-capacity <code>Memory</code>, <code>place in</code>, Task/Channel pipeline, thread-local runtime contexts и безопасная граница между region-owned данными и сообщениями.</li></ul></li>
  <li><code>bench_13_time_budget.skd</code><ul><li>Измерение небольшого time budget в <code>v1.2</code>.</li><li>Покрытие: <code>Time</code>, <code>Duration</code>, literals <code>ms/s</code>, <code>now</code>, <code>elapsed</code>, <code>sleep</code> и передача <code>Duration</code> через <code>Task(Duration)</code>.</li></ul></li>
</ol>

## Репозиторные входные данные

Для воспроизводимого smoke-прогона рядом с showcase лежат стабильные входные данные:

- `benchmarks/showcase-data/sample_weather.txt` для `bench_02_read_stats.skd` и `bench_03_find_count.skd`;
- `benchmarks/showcase-data/tree_fixture/` для `bench_01_tree.skd` и `bench_08_path_list_helpers.skd`.

Это позволяет гонять showcase не на случайном состоянии репозитория, а на фиксированных примерах.

## Сборка всех showcase-программ

Из корня репозитория используйте release smoke-скрипт:

```powershell
.\scripts\run_showcase.ps1 -Mode build
```

```bash
./scripts/run_showcase.sh build
```

Обычные проекты собираются через `skadi-cli check/build/run`. Showcase-файлы
остаются репозиторным smoke-набором и запускаются скриптом, который подбирает для
них стабильные fixtures и ожидаемые аргументы.

Практические правила Task/Channel и пример запуска нескольких workers описаны в
[руководстве по многопоточности](concurrency.md).

## Smoke-запуск

```powershell
Push-Location benchmarks\showcase-data\tree_fixture
..\..\bench_01_tree.exe --dirs-only --depth-1
..\..\bench_08_path_list_helpers.exe
Pop-Location

.\bench_02_read_stats.exe --input benchmarks/showcase-data/sample_weather.txt
.\bench_03_find_count.exe --input benchmarks/showcase-data/sample_weather.txt --needle temperature
.\bench_04_sum_ints.exe --small
.\bench_05_push_pop.exe --small
.\bench_06_struct_account.exe
.\bench_07_struct_list.exe
.\bench_09_math_navigation.exe
.\bench_10_v1_1_toolbox.exe
.\bench_11_task_channel_pipeline.exe
.\bench_12_systems_pipeline.exe
.\bench_13_time_budget.exe
```

Или через вспомогательные скрипты:

```powershell
.\scripts\run_showcase.ps1 -Mode smoke
```

```bash
./scripts/run_showcase.sh smoke
```

Сборка и smoke-проверка одним проходом:

```powershell
.\scripts\run_showcase.ps1 -Mode all
```

```bash
./scripts/run_showcase.sh all
```

## Замечания

- `run_showcase.ps1` и `run_showcase.sh` проверяют, что каждый ожидаемый `.exe` действительно создан.
- Если вызов компилятора завершается ошибкой, скрипт завершается с ненулевым кодом и сообщает, какие showcase-программы не прошли.
- Для `bench_01` и `bench_08` smoke-скрипты временно меняют рабочую директорию на `benchmarks/showcase-data/tree_fixture`.
- Для `bench_02` и `bench_03` smoke-скрипты используют `benchmarks/showcase-data/sample_weather.txt`.
- В `v1.1` проверка showcase-программ остаётся CLI/script-driven, но теперь опирается на репозиторные fixture-данные.
- `skadi-cli tui` можно использовать внутри showcase-проекта для ручного `check`, `build` и `run`, но отдельного браузера showcase-программ в TUI пока нет.

## Зачем нужен именно этот набор

- Он покрывает разные формы синтаксиса и runtime-пути, а не один демонстрационный сценарий.
- Его достаточно быстро гонять после изменений компилятора.
- Он может служить витриной практического стиля кода на Skadi.
