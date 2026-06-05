# Руководство по стилю диагностических сообщений

Этот документ определяет канонический user-facing формат diagnostics для стадий компилятора Skadi.

## Канонический формат

`<Kind> error at line <L>, col <C>[, index <I>]: [<CODE>] <message>`

Где:

- `<Kind>` — один из: `Lex`, `Parse`, `Semantic`
- `<CODE>` — опционален, но желателен (`SC-LEX-001`, `SC-PARSE-003`, `SC-SEM-020`)
- `line`/`col` — 1-based source coordinates, когда они доступны
- `index` — опциональный token index (сейчас используется в parser entry diagnostics)
- `<message>` — короткое, прикладное и точное описание ошибки

Если location data недоступны:

`<Kind> error: [<CODE>] <message>`

## Соглашения по сообщениям

- Начинать с lowercase, если только не используется имя языкового символа (`ErrorCode`, `Int` и т.д.).
- Предпочитать domain-specific phrasing:

  - `use-before-definition`
  - `type mismatch in assignment`
  - `unknown function 'foo'`
- Связанные имена символов брать в одинарные кавычки.
- Не допускать stack-trace style или внутренний debug noise в user-facing сообщениях.

## Примеры

- `Lex error at line 3, col 12: [SC-LEX-001] unexpected character '@'`
- `Parse error at line 5, col 1, index 14: [SC-PARSE-003] expected '{' after 'if' condition.`
- `Semantic error at line 9, col 7: [SC-SEM-020] type mismatch in assignment to 'x': cannot assign Bool to Int.`

## Карта semantic codes (текущее состояние)

- `SC-SEM-010` — redeclaration in scope
- `SC-SEM-011` — invalid self-referential initialization
- `SC-SEM-012` — use-before-definition
- `SC-SEM-020` — type mismatch
- `SC-SEM-030` — unknown function
- `SC-SEM-031` — argument count mismatch
- `SC-SEM-032` — argument type mismatch
- `SC-SEM-040` — invalid semantic context
- `SC-SEM-050` — return-path/return-form rule
- `SC-SEM-051` — `ErrorCode` label/variant rule
- `SC-SEM-900` — internal semantic consistency error

## Карта parse codes (текущее состояние)

- `SC-PARSE-001..003` — parser entry/wrapper diagnostics (`parser/mod.rs`)
- `SC-PARSE-101..148` — statement-level parser diagnostics (`parser/statements.rs`)
- `SC-PARSE-201..206` — expression parser diagnostics (`parser/expressions.rs`)

### Диапазоны parse codes

- `SC-PARSE-10x` — block/function structure и ожидания сигнатур
- `SC-PARSE-11x` — structural expectations для loop/when
- `SC-PARSE-12x` — ожидания для `if/while/loop/return`
- `SC-PARSE-13x` — shape expectations для assignment и danger-call
- `SC-PARSE-14x` — structural expectations для declaration/on-block
- `SC-PARSE-20x` — ожидания expression parser и unexpected tokens
