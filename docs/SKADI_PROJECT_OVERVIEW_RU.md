# Обзор проекта Skadi

Дата обновления: 2026-06-05

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

## 8. Что пока не стоит считать завершённым продуктовым слоем

- imports / modules;
- memory model implementation;
- task/concurrency implementation;
- visual core;
- systems additions;
- законченная семантика выполнения для `on interrupt`.

## 9. Навигация по документам

- [Пользовательские документы](docs/SKADI_DOCS_USER_RU.md) - пользовательская документация
- [Внутренние документы разработки](docs/SKADI_DOCS_INTERNAL_RU.md) - внутренняя документация разработки
