# Scadi: Техническая Документация Проекта (RU)

Дата актуальности: 2026-05-22  
Версия проекта: текущий `master` (прототип v0.1)

## 1. Как это работает в общих чертах

Компиляторный поток сейчас:
1. `main.rs` читает входной `.txt`/`.scadi` файл.
2. `lexer::lex` превращает текст в `Vec<Token>`.
3. `parser::parse_program` строит AST (`Program` + `Statement`/`Expression`).
4. `semantic_analysis::semantic_analyze` валидирует программу по типам и контекстам.
5. `codegen::transpile_program_to_c` генерирует C-код.

Это не backend машинного кода, а транспиляция в C с минимальным runtime-слоем для `List`/`Text`.

## 2. Структура проекта

Корень:
- `Cargo.toml` — сборка и зависимости Rust crate.
- `README_PROJECT_OVERVIEW.md` — краткий обзор проекта.
- `SCADI_IMPLEMENTATION_PLAN.md` — дорожная карта.
- `Scadi_design.txt` — дизайн-языка (источник идей/целей).
- `docs/` — RFC, покрытие, стиль, и эта документация.
- `tests/` — unit/smoke/integration/e2e тесты.

`src/`:
- `lib.rs` — экспорт модулей библиотеки.
- `main.rs` — CLI-точка входа.
- `common_types.rs` — token-контракты и лексические типы.
- `diagnostics.rs` — унифицированный формат диагностик.
- `ast_nodes.rs` — AST-узлы и `ScopeManager`.
- `lexer/` — лексер.
- `parser/` — парсер.
- `semantic_analysis.rs` — семантический анализ.
- `codegen/` — генерация C.

## 3. Модули и файлы (подробно)

### 3.1 `src/lib.rs`

Назначение:
- объявляет и реэкспортирует основные модули (`lexer`, `parser`, `semantic_analysis`, `codegen`, и т.д.).

Роль:
- библиотечная “склейка” проекта для использования из `main.rs` и тестов.

### 3.2 `src/main.rs`

Назначение:
- CLI-раннер компиляторного пайплайна.

Поддерживаемые флаги:
- `--input <path>`
- `--emit-c <path>`
- `--print-c`

Поведение:
- читает исходник,
- запускает `lex -> parse -> semantic -> transpile_to_c`,
- печатает диагностики при ошибках.

### 3.3 `src/common_types.rs`

Назначение:
- типы токенов и структура токена.

Ключевые сущности:
- `TokenKind` — классификация токенов (keywords, operators, literals, punctuation, etc.).
- `Token` — `kind + lexeme + line + col`.
- `LexError` (legacy-форма, фактически проект использует `lexer/structures.rs::LexError`).

### 3.4 `src/diagnostics.rs`

Назначение:
- единый формат сообщений ошибок.

Ключевые сущности:
- `DiagnosticKind` (`Lex`, `Parse`, `Semantic`).
- `format_diagnostic(...)` — стандартная сборка текста диагностики с кодом, координатами и индексом.

### 3.5 `src/ast_nodes.rs`

Назначение:
- AST-контракты и базовый `ScopeManager`.

Ключевые сущности:
- `Location`
- `Program`
- `Statement` (VarDecl, Assignment, FunctionDef, If/While/Loop/For/When, OnBlock, Danger*OnError, ListPush/ListPopOnError, Return*, LabelDecl, StructDecl, ...)
- `Expression` (Literal*, VariableReference, Call, BinaryOp, Index, ListLiteral, StructConstruction)
- `BlockStatement`
- `ScopeManager` (лексическая область видимости, используется парсером ограниченно как scaffold)

### 3.6 `src/lexer/mod.rs`

Назначение:
- модульная точка входа для лексера.

Что делает:
- подключает `core` и `structures`,
- реэкспортирует `lex`.

### 3.7 `src/lexer/structures.rs`

Назначение:
- структуры и helper-ы лексера.

Ключевые элементы:
- `LexError` с Display через `format_diagnostic`.
- `is_operator_start(c)` — быстрый фильтр для потенциальных операторов/пунктуации.

### 3.8 `src/lexer/core.rs`

Назначение:
- полная реализация токенизации.

Что внутри:
- `Lexer`-итератор по символам,
- распознавание:
  - чисел, идентификаторов, keyword-ов,
  - строк,
  - операторов и пунктуации,
  - комментариев (`//`, `/* ... */`),
- функция `lex(source) -> Result<Vec<Token>, LexError>`.

### 3.9 `src/parser/mod.rs`

Назначение:
- orchestration-парсер.

Что делает:
- выбирает нужный statement parser по стартовому токену,
- пропускает whitespace/newline,
- собирает `Program`.

### 3.10 `src/parser/expressions.rs`

Назначение:
- Pratt-парсер выражений.

Возможности:
- префиксные и инфиксные операторы с приоритетами,
- вызовы функций,
- индексация,
- list literal,
- базовые литералы и идентификаторы.

### 3.11 `src/parser/statements.rs`

Назначение:
- парсинг деклараций и инструкций верхнего уровня/блоков.

Покрывает:
- `fn` / `danger fn`,
- `if`, `while`, `loop`,
- `for in` + `iterate ... as ...`,
- `when`,
- `return` и `return error`,
- `new` declarations,
- `label`, `struct`,
- `on ...` блоки,
- специальные формы:
  - `x = parse(...) on error { ... }`,
  - `x = xs.pop() on error { ... }`,
  - `xs.push(v)`.

### 3.12 `src/semantic_analysis.rs`

Назначение:
- типизация и контекстные проверки AST.

Ключевые проверки:
- scope rules (`use-before-definition`, redeclaration),
- type compatibility,
- function call args/signatures,
- `danger/on error` validity,
- `ErrorCode` conventions,
- `List`/`Text` builtin типы,
- проверка условий `if/while` на bool,
- проверка завершения `danger fn` return-ами.

### 3.13 `src/codegen/mod.rs`

Назначение:
- модульная точка входа codegen.

Что делает:
- реэкспорт `transpile_program_to_c`.

### 3.14 `src/codegen/c.rs`

Назначение:
- основной C-lowering.

Что делает:
- собирает include-ы,
- генерирует runtime helper-ы:
  - `List`: new/push/pop/get,
  - `Text`: char_at/find/slice,
- lower-ит:
  - функции,
  - var/assignment,
  - control-flow,
  - when->if-chain,
  - builtins `len/contains/find/slice`,
  - list/text indexing.

## 4. Каталог функций (каждая функция кратко)

Ниже — краткое назначение каждой функции в `src/` (по состоянию на эту дату).

### 4.1 `src/main.rs`

- `main` — CLI-пайплайн: аргументы, чтение файла, запуск этапов компиляции, вывод результатов.

### 4.2 `src/ast_nodes.rs`

- `Location::default` — координаты по умолчанию.
- `Program::new` — создать пустую программу.
- `From<Vec<Statement>> for Box<BlockStatement>::from` — удобная упаковка списка statements в block.
- `ScopeManager::new` — создать менеджер областей.
- `ScopeManager::enter_scope` — войти в новую область.
- `ScopeManager::exit_scope` — выйти из текущей области.
- `ScopeManager::define_symbol` — объявить символ в текущей области.
- `ScopeManager::lookup` — найти символ от локальной области к внешним.

### 4.3 `src/common_types.rs`

- `Token::kind` — вернуть kind токена (clone).
- `LexError::fmt` — строковое представление лексической ошибки.

### 4.4 `src/diagnostics.rs`

- `DiagnosticKind::as_str` — название категории диагностики.
- `format_diagnostic` — унифицированный формат ошибок/диагностик.

### 4.5 `src/lexer/structures.rs`

- `LexError::fmt` — форматированный вывод лексической ошибки.
- `is_operator_start` — проверка, может ли символ начинать оператор/пунктуацию.

### 4.6 `src/lexer/core.rs`

- `Lexer::new` — инициализация лексера.
- `Lexer::peek` — текущий символ без сдвига.
- `Lexer::peek_next` — следующий символ без сдвига.
- `Lexer::advance` — сдвиг на символ.
- `Lexer::has_more` — есть ли ещё символы.
- `Lexer::starts_with` — проверка префикса от текущей позиции.
- `Lexer::lexeme_from_range` — собрать строку по диапазону символов.
- `Lexer::skip_line_comment` — пропустить `// ...`.
- `Lexer::skip_block_comment` — пропустить `/* ... */`.
- `Lexer::scan_string_literal` — собрать строковый литерал.
- `Lexer::scan_number` — собрать числовой литерал.
- `Lexer::scan_identifier` — собрать идентификатор/keyword.
- `Lexer::resolve_keyword` — классификация идентификатора как keyword/type/bool/identifier.
- `Lexer::next_token` — получить следующий токен или лексическую ошибку.
- `Lexer::next` — реализация `Iterator` для потока токенов.
- `lex` — публичный API лексера.

### 4.7 `src/parser/mod.rs`

- `parse_statement_at` — выбрать и вызвать нужный parser по стартовому токену.
- `parse_statements_range` — распарсить диапазон токенов в список statements.
- `parse_program` — распарсить весь поток токенов в `Program`.

### 4.8 `src/parser/expressions.rs`

- `parse_err` — helper форматирования parser-error.
- `PrattParser::new` — создать Pratt-парсер выражения.
- `PrattParser::parse` — вход в парсинг выражения.
- `PrattParser::parse_bp` — Pratt-ядро с binding powers.
- `PrattParser::parse_prefix` — разбор префиксной/атомарной части.
- `is_infix_operator` — является ли токен инфикс-оператором.
- `infix_binding_power` — приоритет/ассоциативность оператора.
- `parse_expression_range` — публичный разбор выражения на диапазоне токенов.

### 4.9 `src/parser/statements.rs`

- `parse_err` — helper parser-error.
- `parse_expression_list` — разобрать список выражений (например args/cases).
- `find_block_end` — найти закрывающую `}` для блока.
- `parse_function_declaration` — разобрать `fn`/`danger fn`.
- `parse_for_loop` — разобрать `for item in collection`.
- `parse_iterate_loop` — разобрать `iterate collection as item`.
- `parse_when_statement` — разобрать `when/is/else`.
- `parse_if_statement` — разобрать `if/else`.
- `parse_while_statement` — разобрать `while`.
- `parse_loop_statement` — разобрать `loop`.
- `parse_return_statement` — разобрать `return` и `return error`.
- `parse_assignment_statement` — разобрать присваивание.
- `parse_call_expression` — разобрать call-подвыражение в спец-контекстах.
- `parse_identifier_led_statement` — обработка statement-ов, начинающихся с идентификатора.
- `parse_new_declaration` — разобрать `new ...`.
- `parse_label_declaration` — разобрать `label`.
- `parse_struct_declaration` — разобрать `struct`.
- `parse_on_block_statement` — разобрать `on ...` блок.

### 4.10 `src/semantic_analysis.rs`

- `statement_loc` — достать координаты statement.
- `sem_err` — собрать semantic-error без привязки к statement.
- `err_at_code` — собрать semantic-error с координатами statement.
- `semantic_analyze` — главный вход семантического анализа.
- `validate_error_code_label` — проверить базовые правила `label ErrorCode`.
- `parse_primitive_type_name` — маппинг примитивного типа в `ValueType`.
- `parse_type_name` — маппинг составного type-string (`... List`) в `ValueType`.
- `can_assign` — проверка совместимости присваивания.
- `validate_call_args` — проверка количества/типов аргументов вызова.
- `analyze_statements` — проход по списку statements.
- `analyze_statement` — анализ одного statement.
- `analyze_block` — анализ block.
- `param_type_or_default` — тип параметра функции или default.
- `infer_expression_type` — вывод типа выражения.
- `block_guarantees_termination` — проверка, завершится ли block return/error.
- `statement_guarantees_termination` — проверка терминирования statement.
- `contains_variable` — поиск self-reference в выражении.

### 4.11 `src/codegen/c.rs`

- `list_elem_from_decl` — достать тип элемента из строки типа списка.
- `list_meta` — получить C-type и суффикс runtime-функций списка.
- `emit_list_runtime` — сгенерировать C-runtime для `List`.
- `emit_text_runtime` — сгенерировать C-runtime для `Text`.
- `transpile_program_to_c` — главный вход codegen.
- `program_uses_text_runtime` — нужна ли text-runtime часть (скан AST).
- `program_uses_list_runtime` — нужна ли list-runtime часть (скан AST).
- `emit_error_code_enum` — сгенерировать enum `ErrorCode` из label.
- `emit_function` — lower одного function definition.
- `emit_block` — lower блока statements.
- `emit_statement` — lower одного statement.
- `map_skadi_type_to_c` — маппинг типа Scadi в C-тип.
- `emit_expr` — lower выражения.

## 5. Тестовая подсистема

Ключевые наборы:
- `lexer_smoke.rs` — базовые проверки лексера.
- `parser_smoke.rs` — позитивные parser-сценарии.
- `parser_negative.rs` — негативные parser-сценарии.
- `semantic_smoke.rs` — позитив/негатив семантики.
- `codegen_smoke.rs` — shape-тесты C-lowering.
- `language_programs.rs` — интеграционные “мини-программы”.
- `conformance_suite.rs` — системная проверка core-конструкций.
- `codegen_e2e.rs` — компиляция сгенерированного C внешним компилятором и запуск бинарника.

## 6. Текущее состояние зрелости

Готово:
- стабильный базовый пайплайн,
- развитый тестовый контур,
- рабочий C-transpile для core-подмножества.

Не завершено:
- полноценная runtime-модель событий/конкурентности,
- полное lowering для struct/advanced memory model,
- финальный контракт error-flow для индексации (сейчас fail-soft runtime fallback).
