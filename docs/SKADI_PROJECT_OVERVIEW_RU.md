# Обзор проекта Skadi

Дата обновления: 2026-06-21

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
- showcase-программы и набор регрессионных тестов;
- HTML-сайт документации на базе `MkDocs` с каркасом RU/EN.

## 3. Как теперь устроена документация

Пользовательские документы:

- [Пользовательские документы](docs/SKADI_DOCS_USER_RU.md)

Внутренние документы разработки языка и компилятора:

- [Внутренние документы разработки](docs/SKADI_DOCS_INTERNAL_RU.md)

Если нужен быстрый маршрут без выбора:

- начать с [пользовательских документов](docs/SKADI_DOCS_USER_RU.md), если цель писать программы на Skadi;
- начать с [внутренних документов](docs/SKADI_DOCS_INTERNAL_RU.md), если цель развивать язык, компилятор или runtime-контракты.

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
workflow [docs-pages.yml](.github/workflows/docs-pages.yml), который собирает
`site/` и публикует его через GitHub Pages.

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

- Memory MVP уже проходит parser/semantic/codegen/runtime путь для strict fixed-capacity surface.
- Task/Channel MVP проходит parser/semantic/codegen/runtime путь: native `run/wait`,
  cooperative `stop/stopping` и bounded blocking Channel работают на Win32/pthread.
  Слой остаётся experimental до успешной проверки dedicated TSan и
  GCC/Clang/MinGW/MSVC jobs в release CI.

Пользовательский контракт и практические шаблоны собраны в
[руководстве по многопоточности](concurrency.md).

Подробная рамка находится в [Плане v1.2](docs/SKADI_V1_2_PLAN_RU.md).

## 9. Что пока не стоит считать завершённым продуктовым слоем

- imports / modules;
- расширенные memory policies: `allow grow`, `allow drop`, `memory.child`, `memory.static`;
- расширенный concurrency surface: `close`, cancellation, timeout, `select`, task groups;
- visual core;
- systems additions;
- законченная семантика выполнения для `on interrupt`.

## 10. Навигация по документам

- [Пользовательские документы](docs/SKADI_DOCS_USER_RU.md) - пользовательская документация
- [Внутренние документы разработки](docs/SKADI_DOCS_INTERNAL_RU.md) - внутренняя документация разработки
