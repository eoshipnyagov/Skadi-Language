# Skadi: Внутренняя документация разработки (RU)

Этот документ собирает внутренние материалы по разработке языка, компилятора,
текущих experimental tracks и будущих архитектурных направлений.

Эти материалы также входят в HTML-сайт документации, но логически отделены от
пользовательского слоя.

Сюда входят документы для:

- разработки parser / semantic / codegen;
- фиксации контрактов и ограничений;
- планирования текущих и будущих языковых треков;
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
  - стиль, форма и code ranges диагностических сообщений.

- [Справочник кодов диагностики](diagnostic-codes.md)
  - актуальные семейства `SC-LEX`, `SC-PARSE`, `SC-SEM`, `SC-MOD` и `SC-CGEN`.

- [Матрица токенов и конструкций](token-construct-coverage.md)
  - трассировка конструкций через lexer, parser, semantic, codegen и e2e.

- [Scope и видимость v1.1](scope-visibility.md)
  - контракт `local`, `hide`, direct imports и квалифицированных имён.

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
  - stable/productization план `v1.1`.

- [Закрытие roadmap v1.1](v1-1-roadmap.md)
  - исторический close-out scope/visibility и CI-гейтов `v1.1`.

- [План v1.2](v1-2-plan.md)
  - текущая рабочая линия после `v1.1`: Memory MVP и Task/Channel runtime track.

- [Блокеры v1](v1-blockers.md)
  - блокеры и несогласованности.

- [Backlog math/vector](math-vector-backlog.md)
  - backlog по развитию math/vector направления.

- [Сайт документации и i18n](docs-site-and-i18n.md)
  - устройство HTML-сайта документации и двуязычного контура.

## 4. Текущие experimental tracks `v1.2`

### Memory

- [Memory Draft](memory-model-draft.md)
- [Memory MVP Contract](memory-model-mvp.md)
- [Примеры и антипримеры Memory](memory-model-examples.md)

### Task / concurrency

- [Task Draft](task-model-draft.md)
- [Task MVP Contract](task-model-mvp.md)
- [Task Runtime MVP Design](task-runtime-mvp-design.md)

## 5. Future tracks

### Visual core

- [Visual Core Draft](visual-core-draft.md)
- [Visual Core MVP Contract](visual-core-mvp.md)

### Systems additions

- [Systems Additions Draft](systems-additions-draft.md)
- [Systems Additions MVP Contract](systems-additions-mvp.md)

## 6. Что читать в типичных ситуациях

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
