# Showcase-программы Skadi

В этом разделе собраны 9 небольших showcase-программ.

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
</ol>

## Сборка всех showcase-программ

Из корня репозитория:

```powershell
cargo run -- --input benchmarks/bench_01_tree.skd --emit-exe bench_01_tree.exe
cargo run -- --input benchmarks/bench_02_read_stats.skd --emit-exe bench_02_read_stats.exe
cargo run -- --input benchmarks/bench_03_find_count.skd --emit-exe bench_03_find_count.exe
cargo run -- --input benchmarks/bench_04_sum_ints.skd --emit-exe bench_04_sum_ints.exe
cargo run -- --input benchmarks/bench_05_push_pop.skd --emit-exe bench_05_push_pop.exe
cargo run -- --input benchmarks/bench_06_struct_account.skd --emit-exe bench_06_struct_account.exe
cargo run -- --input benchmarks/bench_07_struct_list.skd --emit-exe bench_07_struct_list.exe
cargo run -- --input benchmarks/bench_08_path_list_helpers.skd --emit-exe bench_08_path_list_helpers.exe
cargo run -- --input benchmarks/bench_09_math_navigation.skd --emit-exe bench_09_math_navigation.exe
```

Или через вспомогательный скрипт:

```powershell
.\scripts\run_showcase.ps1 -Mode build
```

## Smoke-запуск

```powershell
.\bench_01_tree.exe --dirs-only --depth-3
.\bench_02_read_stats.exe --input examples/example_meteostation.skd
.\bench_03_find_count.exe --input examples/example_meteostation.skd --needle temperature
.\bench_04_sum_ints.exe --medium
.\bench_05_push_pop.exe --medium
.\bench_06_struct_account.exe
.\bench_07_struct_list.exe
.\bench_08_path_list_helpers.exe
.\bench_09_math_navigation.exe
```

Или через вспомогательный скрипт:

```powershell
.\scripts\run_showcase.ps1 -Mode smoke
```

Сборка и smoke-проверка одним проходом:

```powershell
.\scripts\run_showcase.ps1 -Mode all
```

## Замечания

- `run_showcase.ps1` проверяет, что каждый ожидаемый `.exe` действительно создан.
- Если вызов компилятора завершается ошибкой, скрипт завершается с ненулевым кодом и сообщает, какие showcase-программы не прошли.
- В `v1.1` проверка showcase-программ остаётся CLI/script-driven.
- `skadi-cli tui` можно использовать внутри showcase-проекта для ручного `check`, `build` и `run`, но отдельного браузера showcase-программ в TUI пока нет.

## Зачем нужен именно этот набор

- Он покрывает разные формы синтаксиса и runtime-пути, а не один демонстрационный сценарий.
- Его достаточно быстро гонять после изменений компилятора.
- Он может служить витриной практического стиля кода на Skadi.
