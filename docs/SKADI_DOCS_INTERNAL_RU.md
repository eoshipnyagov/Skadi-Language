# Skadi: Внутренняя документация разработки (RU)

Этот документ собирает внутренние материалы по разработке языка, компилятора и
будущих архитектурных треков.

Эти материалы также входят в HTML-сайт документации, но логически отделены от
пользовательского слоя.

Сюда входят документы для:

- разработки parser / semantic / codegen;
- фиксации контрактов и ограничений;
- планирования будущих языковых треков;
- синхронизации дизайна языка с реализацией.

Пользовательские документы вынесены отдельно:

- [Пользовательские документы](../user/index.md)

## 1. Реализация и текущее состояние компилятора

- [Техсправочник проекта](project-tech-reference.md)
  - технический обзор проекта.

- [Границы Skadi -> C](to-c-scope.md)
  - границы и контракт текущего `Skadi -> C` backend.

- [Покрытие тестами](test-coverage.md)
  - что реально покрыто тестами.

- [Стиль диагностики](diagnostics-style.md)
  - стиль и форма диагностических сообщений.

- `CLI_USAGE.md`
  - служебная заметка про пользовательские входы и низкоуровневый driver.

## 2. Контракты текущего `v1`

- [Контракт Text v1](text-contract-v1.md)
- [RFC Text](rfc-text.md)
- [RFC List](rfc-list.md)
- [Матрица on error v1](on-error-v1.md)
- [Контракт runtime memory v1](c-runtime-memory-contract-v1.md)
- [Каноническая матрица синтаксиса](syntax-canonical-matrix.md)
- [Стилевые принципы](style-principles.md)
- [Style Guide v1](style-guide-v1.md)

## 3. Планы и релизный контур

- [План реализации](implementation-plan.md)
  - общий рабочий план.

- [План v1.1](v1-1-plan.md)
  - рабочий план `v1.1`.

- [Блокеры v1](v1-blockers.md)
  - блокеры и несогласованности.

- [Backlog math/vector](math-vector-backlog.md)
  - backlog по развитию math/vector направления.

- [Сайт документации и i18n](docs-site-and-i18n.md)
  - устройство HTML-сайта документации и двуязычного контура.

## 4. Future tracks

### Memory

- [Memory Draft](memory-model-draft.md)
- [Memory MVP Contract](memory-model-mvp.md)

### Task / concurrency

- [Task Draft](task-model-draft.md)
- [Task MVP Contract](task-model-mvp.md)

### Visual core

- [Visual Core Draft](visual-core-draft.md)
- [Visual Core MVP Contract](visual-core-mvp.md)

### Systems additions

- [Systems Additions Draft](systems-additions-draft.md)
- [Systems Additions MVP Contract](systems-additions-mvp.md)

## 5. Что читать в типичных ситуациях

Если работа идёт по текущему компилятору:

- [Техсправочник проекта](project-tech-reference.md)
- [Границы Skadi -> C](to-c-scope.md)
- [Покрытие тестами](test-coverage.md)

Если нужно проверить реальный контракт языка:

- [Статус синтаксиса](syntax-status.md)
- [Справочник языка](language-reference.md)
- документы из раздела контрактов `v1`

Если работа идёт по будущим трекам:

- соответствующий `*_DRAFT_RU.md`
- затем `*_MVP_CONTRACT_RU.md`
