# Skadi Memory Model: Примеры и антипримеры

Дата: 2026-06-06  
Статус: рабочая опора для experimental frontend MVP.

Связанные документы:

- [Memory MVP Contract](memory-model-mvp.md)
- [Memory Draft](memory-model-draft.md)

## 1. Зачем нужен отдельный набор примеров

Memory model легко сделать визуально "умной", но плохо читаемой.

Поэтому для Skadi здесь принят жёсткий принцип:

> если пример требует скрытого контекста или телепатии, это плохой пример.

Хороший memory-пример должен:

- быть самодостаточным;
- показывать одну главную идею;
- использовать понятные имена вроде `assets_memory`, `scratch_memory`, `file_text`, `batch_result`;
- не опираться на неочевидные доменные сущности вроде `Scene`, если они не объявлены рядом;
- не использовать в канонических местах схлопнутые формы вроде `{raw}` или `{raw = raw}`.

## 2. Как читать этот набор

Есть три категории файлов:

- `examples/memory/positive/`
  - корректные backend-enabled MVP примеры;
  - они должны проходить parser, semantic, `Skadi -> C` и native build/run path.

- `examples/memory/pitfalls/`
  - валидные программы, которые intentionally показывают плохой стиль;
  - они должны проходить semantic, но emit'ить style warnings.

- `examples/memory/negative/`
  - намеренно запрещённые сценарии;
  - они должны падать явной diagnostics уже на semantic stage.

## 3. Канонические positive examples

### 3.1. Возврат значения через внешнюю `Memory`

Файл: `examples/memory/positive/01_loaded_text_asset.skd`

Что показывает:

- `Memory` передана в функцию извне;
- `place in` размещает dynamic payload в этой памяти;
- вернуть такое значение можно;
- пример сам объясняет себя через `LoadedText` и `content`.

```scadi
struct LoadedText {
    Text content
}

fn load_text(Memory assets_memory, Path path) LoadedText {
    place in assets_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

### 3.2. Локальная scratch-memory для временных данных

Файл: `examples/memory/positive/02_local_scratch_preview.skd`

Что показывает:

- локальная `Memory` подходит для временной работы;
- значение не утекает наружу;
- `clear()` вызывается после блока и в trailing `on error`, а не внутри active `place in`.

### 3.3. Возврат `List` из внешней `Memory`

Файл: `examples/memory/positive/03_sensor_batch_external_memory.skd`

Что показывает:

- region-relevant payload это не только `Text`, но и `List`;
- значение со списком можно вернуть, если `Memory` передана в функцию извне;
- self-contained имена лучше неочевидных доменных абстракций.

### 3.4. Явное восстановление после overflow

Файл: `examples/memory/positive/04_explicit_recovery.skd`

Что показывает:

- `place in ... on error ...` не обещает неявный rollback;
- если нужен чистый регион, пользователь делает это явно через `assets_memory.clear()`;
- fallback path должен быть таким же читаемым и самодостаточным, как happy path.

## 4. Style pitfalls

### 4.1. Схлопнутые имена полей и переменных

Файл: `examples/memory/pitfalls/01_collapsed_field_names.skd`

Этот пример валиден, но считается плохим каноном:

```scadi
new Text content = read(path)
new LoadedText result = {content}
```

Почему это плохо:

- читатель вынужден держать в голове, где поле, а где локальная переменная;
- форма хуже сканируется глазами;
- для AI-assisted codegen это повышает риск неверной интерпретации.

Поэтому compiler теперь даёт style warning и предлагает отдельные имена вроде:

- `file_text`
- `content_text`
- `config_value`
- `batch_result`

## 5. Negative examples и ожидаемые diagnostics

### 5.1. Нельзя вернуть значение из локальной `Memory`

Файл: `examples/memory/negative/01_local_memory_return_escape.skd`

Ожидаемый diagnostic:

- `SC-SEM-061`

### 5.2. Нельзя вызывать `clear()` внутри активного `place in` того же региона

Файл: `examples/memory/negative/02_in_block_clear.skd`

Ожидаемый diagnostic:

- `SC-SEM-060`

### 5.3. Нельзя хранить `Memory` в `struct`

Файл: `examples/memory/negative/03_memory_in_struct.skd`

Ожидаемый diagnostic:

- `SC-SEM-062`

### 5.4. Нельзя делать `Memory List`

Файл: `examples/memory/negative/04_memory_list.skd`

Ожидаемый diagnostic:

- `SC-SEM-062`

### 5.5. Нельзя копировать или переприсваивать `Memory`

Файл: `examples/memory/negative/05_memory_copy_assignment.skd`

Ожидаемый diagnostic:

- `SC-SEM-062`

### 5.6. Obvious use-after-clear должен ловиться явно

Файл: `examples/memory/negative/06_use_after_clear.skd`

Ожидаемый diagnostic:

- `SC-SEM-061`

### 5.7. Нельзя сохранять local-region payload в более долгоживущий owner

Файл: `examples/memory/negative/07_store_into_longer_lived_owner.skd`

Ожидаемый diagnostic:

- `SC-SEM-060`

## 6. Что считать каноническим стилем memory-кода

Для current MVP канон такой:

- `Memory` остаётся отдельным capability-like surface;
- имена region handles заканчиваются на `_memory`;
- `place in` сначала показывает основной блок, а `on error` идёт trailing-веткой;
- `clear()` используется после placement-блока или в trailing `on error`;
- поле и локальная переменная по возможности не должны называться одинаково;
- если пример про memory, он не должен одновременно требовать знания скрытой бизнес-модели.

## 7. Практическое правило для новых примеров

Перед тем как добавить новый memory example, стоит проверить три вопроса:

1. Поймёт ли человек этот пример, не зная проектный lore?
2. Показывает ли пример одну идею, а не пять сразу?
3. Если нейросеть повторит этот стиль буквально, получится ли хороший Skadi-код?

Если ответ хотя бы на один из них "нет", пример ещё не канонический.
