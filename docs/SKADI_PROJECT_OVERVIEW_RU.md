# Обзор проекта Skadi

Дата обновления: 2026-07-30

## 1. Что это за репозиторий

Это рабочий Rust-прототип языка Skadi. Текущий практический контур проекта:

<pre><code class="language-text">Skadi source -&gt; lexer -&gt; parser -&gt; semantic -&gt; C codegen -&gt; C compiler -&gt; binary</code></pre>

Главная цель текущего этапа - не финальный native backend, а стабильный,
тестируемый и удобный `Skadi -> C` pipeline с нормальным пользовательским UX.

## 2. Что уже есть на текущий момент

Реализованы и используются:

- lexer с диагностикой;
- parser с покрытием core syntax;
- semantic analysis с типовыми `SC-SEM-*` diagnostics;
- C codegen;
- `skadi-cli` как канонический пользовательский интерфейс;
- полноэкранный `skadi-cli tui` как полноценный интерактивный путь работы;
- formatter для текущего слоя `v1.1`;
- math/core срез `v1.1`;
- strict Memory MVP как experimental `v1.2` systems layer;
- Task/Channel runtime MVP как experimental `v1.2` systems layer;
- Time/Duration, ByteSize, Angle и bounded Vector runtime MVP;
- typed periodic host Interrupt и строгий `on interrupt` context;
- portable headless Canvas v0 и Win32 presenter;
- showcase-программы и набор регрессионных тестов;
- HTML-сайт документации на базе `MkDocs` с RU/EN user reference;
- release archives и installers для Windows, Linux и macOS.

## 3. Как теперь устроена документация

Пользовательские документы:

- [Пользовательские документы](../user/index.md)

Внутренние документы разработки языка и компилятора:

- [Внутренние документы разработки](index.md)

Если нужен быстрый маршрут без выбора:

- начать с [пользовательских документов](../user/index.md), если цель писать программы на Skadi;
- начать с [внутренних документов](index.md), если цель развивать язык, компилятор или runtime-контракты.

## 4. Ключевые точки входа в код

- `src/main.rs` - низкоуровневый CLI-драйвер компилятора
- `src/lib.rs` - связывание модулей
- `src/lexer/*` - lexer
- `src/parser/*` - parser
- `src/semantic_analysis.rs` - semantic checks
- `src/codegen/c.rs` - C backend
- `src/formatter.rs` - formatter
- `tools/skadi-cli/src/main.rs` - канонический пользовательский CLI
- `tools/skadi-cli/src/tui.rs` - full-screen TUI
- `tests/*` - regression suite

## 5. Как работать с проектом как пользователь

Канонический путь:

<pre><code class="language-bash">skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run</code></pre>

Интерактивный режим:

```bash
skadi-cli tui
```

## 6. HTML-справка

Локально HTML-справку можно открыть отдельным скриптом:

```powershell
scripts\open_docs.ps1
```

Если нужен git-hosted вариант публикации, для этого уже подготовлен
workflow `docs-pages.yml`, который собирает `site/` и публикует его через
GitHub Pages.

## 7. Что считать стабильной частью `v1.1`

С точки зрения пользователя уже можно опираться на:

- базовый язык (`new`, функции, циклы, `when`, `danger fn`, `on error`);
- `Text`, `Path`, `List`;
- `struct` и методы;
- math core;
- `check/build/run/format/doctor`;
- работа через TUI;
- showcase-программы как ориентир по стилю и реальным сценариям.

## 8. Что считать текущей рабочей линией `v1.2`

`v1.2` развивается поверх stable base `v1.1`.

- Memory runtime проходит parser/semantic/codegen/runtime путь для fixed,
  segmented-growing, child и root-static regions.
- `view`/`direct` borrows и `move` ownership transfer работают для текущих
  linear resources.
- Task/Channel MVP проходит parser/semantic/codegen/runtime путь: native `run/wait`,
  cooperative `stop/stopping`, bounded blocking Channel, `try_send` и
  close/drain работают на Win32/pthread; `stop` пробуждает блокирующий
  `send/receive`, не закрывая Channel.
- Time/Duration MVP проходит parser/semantic/codegen/runtime путь: nominal-типы,
  literals `ms/s/min` и monotonic Win32/POSIX runtime исполняются end-to-end.
- Dedicated TSan и GCC/Clang/MinGW/MSVC jobs в remote CI проходят; systems API
  остаётся experimental из-за незамороженных контрактов, а не отсутствия backend.

Пользовательский контракт и практические шаблоны собраны в
[руководстве по многопоточности](../user/concurrency.md).

Подробная рамка находится в [Плане v1.2](v1-2-plan.md).

## 9. Что пока не стоит считать завершённым продуктовым слоем

- module-name imports и re-exports поверх path-imports с рабочими aliases;
- автоматическая reclamation по `allow drop` и полный lifetime calculus;
- `select` и task groups поверх стабилизированных Channel/Task cancellation и
  timeout boundaries;
- Canvas events, text/images, transforms и non-Windows presenters;
- systems additions;
- hardware/RTOS backend для `on interrupt` поверх готового host periodic MVP;
- расширенные path-sensitive ownership/lifecycle facts и Skadi-level debugger в
  TUI; CLI уже поддерживает первый probe-based breakpoint/step slice, а
  structured analysis покрывает blocking Channel и incomplete `when`.

## 10. Навигация по документам

- [Пользовательские документы](../user/index.md) - пользовательская документация
- [Внутренние документы разработки](index.md) - внутренняя документация разработки
