# Skadi Compiler: Project Overview

Дата обновления: 2026-05-22

## 1. Что это за репозиторий

Это рабочий прототип компилятора языка Skadi на Rust.  
Текущая цель этапа: стабильный и тестируемый пайплайн
`lexer -> parser -> semantic -> C transpiler`.

## 2. Текущее состояние

Сейчас реализовано:
- лексический анализ (токенизация, диагностика),
- синтаксический анализ (AST для core-конструкций),
- семантические проверки типов и контекстов,
- генерация C-кода для поддерживаемого подмножества языка,
- развитый набор тестов (smoke, negative, conformance, e2e).

## 3. Куда смотреть в первую очередь

Документация языка (RU):
- `docs/SKADI_LANGUAGE_REFERENCE_RU.md`

Техническая документация проекта (RU):
- `docs/SKADI_PROJECT_TECH_REFERENCE_RU.md`

Дополнительно:
- `docs/TEST_COVERAGE_MATRIX.md` — покрытие тестами,
- `docs/RFC_LIST.md` — контракт `List`,
- `docs/RFC_TEXT.md` — контракт `Text`,
- `docs/RFC_MATH_VECTOR_CORE.md` — математика/векторы/матрицы (целевой v1 трек),
- `docs/SKADI_TO_C_SCOPE.md` — scope transpile в C,
- `SKADI_IMPLEMENTATION_PLAN.md` — roadmap.

## 4. Структура кода

- `src/main.rs` — CLI-вход, запуск пайплайна.
- `src/lib.rs` — wiring модулей.
- `src/lexer/*` — лексер.
- `src/parser/*` — парсер.
- `src/semantic_analysis.rs` — семантическая валидация.
- `src/codegen/c.rs` — транспиляция в C.
- `tests/*` — полный контур тестов.

## 5. Как запустить

Проверка сборки:
```bash
cargo check
```

Все тесты:
```bash
cargo test
```

Запуск пайплайна:
```bash
cargo run -- --input example_meteostation.txt --print-c
```

Запись сгенерированного C в файл:
```bash
cargo run -- --input example_meteostation.txt --emit-c out.c
```

## 6. Ключевые ограничения текущей версии

- Это прототип и транспилятор в C, а не финальный native backend.
- Часть языкового дизайна (из `Skadi_design.txt`) пока не реализована полностью.
- Runtime-поведение ряда операций пока временное (fail-soft для некоторых out-of-range индексаций).

## 7. Ближайший приоритет развития

- завершить и зафиксировать контракт ошибок runtime,
- дорасширить синтаксис/семантику до согласованного v1-подмножества,
- держать “один feature = parser + semantic + codegen + tests + docs”.

