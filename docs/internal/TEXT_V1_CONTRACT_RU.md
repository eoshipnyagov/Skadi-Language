# Skadi v1: Контракт `Text`

Дата: 2026-05-26
Статус: зафиксированный контракт для `v1`.

## Модель данных

- `Text` представлен UTF-8 строкой.
- В текущем v1 runtime большинство операций byte-oriented (не grapheme-aware).

## Поддерживаемые операции

- `len(text)`
- `contains(text, sub)`
- `find(text, sub)`
- `slice(text, start, end)`
- `text[i]`

## Поведение границ

- `slice` нормализует диапазон (`clamp`).
- `text[i]` вне диапазона возвращает fail-soft значение по текущему v1 runtime-контракту.

## Контракт ошибок

- Для `Text`-операций сейчас используется runtime-safe поведение.
- Жесткий переход к universal `on error` для всех text-операций отложен на последующие версии.

## Тесты

Для изменения `Text`-контракта обязательны:
- unit (semantic/type),
- codegen smoke,
- e2e compile+run сценарии.
