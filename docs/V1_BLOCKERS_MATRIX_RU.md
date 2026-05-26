# Skadi v1 Blockers Matrix

Дата: 2026-05-26
Назначение: фиксировать обязательные решения перед стабильным `v1` релизом транспилятора `Skadi -> C`.

## P0 (блокирует v1)

1. Стабильность codegen e2e (feature-mix программы) — В РАБОТЕ
2. Единообразие диагностик parser/semantic/codegen — В РАБОТЕ
3. Multi-file import contract (`import "./... .skd"`) + edge-cases — ЗАКРЫТО
4. Кроссплатформенный CLI pipeline (Win/Linux/macOS) + doctor — В РАБОТЕ

## P1 (можно в v1.x)

1. Расширение `on error` beyond danger/list-pop
2. Расширение struct/method lowering
3. Math/vector core API
4. Offline docs UX (`skadi docs`) и LLM-guide генерация

## Техдолг

- Финальная полировка всех RU/EN доков и перекрестных ссылок
- Дополнительные negative тесты на редкие edge-cases parser/semantic
- Дополнительные invariant-проверки generated C

## Принцип закрытия пункта

Пункт считается закрытым только если есть:
- код,
- тесты,
- синхронное обновление документации.
