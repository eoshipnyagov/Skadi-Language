# RFC: Math + Vector Core (v1.x target)

Status: Draft
Date: 2026-05-26
Owner: Skadi core

## 1. Цель

Добавить в ядро наиболее частые math/vector операции для gamedev/embedded сценариев, не перегружая синтаксис.

## 2. Минимальный набор API

- Константы: `PI`, `E`, `TAU`, `EPSILON`.
- Функции: `sin`, `cos`, `atan2`, `sqrt`, `pow`, `abs`, `min`, `max`, `clamp`.
- Числовые утилиты: `floor`, `ceil`, `round`, `trunc`, `fract`.
- Угол: `normalize_angle(a)`.
- Векторы: `Vec2`, `Vec3`, `Vec4` + поэлементная арифметика и `dot`.

## 3. Ограничения первой итерации

- Без сложного SIMD-specific backend.
- Без перегруженной матричной DSL; 2D-матрицы допускаются как удобные обертки над `List`.
- Все новые функции должны получить: parser + semantic + codegen + e2e.

## 4. Критерий готовности

- API стабилен и документирован.
- Не ломает текущий v1 pipeline.
- Покрыт комбинационными e2e-тестами.
