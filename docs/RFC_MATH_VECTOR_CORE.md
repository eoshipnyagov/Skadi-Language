# RFC: Math + Vector Core

Status: Historical proposal; math core accepted in v1.1, bounded vectors implemented experimentally in v1.2
Date: 2026-05-22
Owner: Skadi core

## 1. Что уже есть в дизайне

В раннем дизайне языка были зафиксированы:

- составные типы `Vec2`, `Vec3`, `Vec4`,
- базовая математика в ядре: `sin`, `cos`, `sqrt`, `abs`, `rand`, `PI`, `E`.

## 2. Что добавляем/уточняем

Добавить в ядро:

- `atan2(y, x)`,
- `root(x, n)` как корень степени `n` (эквивалент `x^(1/n)`),
- явное закрепление `PI`, `E` как встроенных констант.

Уточнение по векторным типам:

- `Vec2`, `Vec3`, `Vec4` остаются встроенными типами с поэлементной арифметикой.

Добавить удобный синтаксис 2D-матрицы:

- матрица как обертка над `List`,
- минимальный v1-синтаксис (предложение):

  - `new Matrix2D m = [[1, 2], [3, 4]]`
  - либо алиас-форма на основе `List(List(Float))`.

## 3. Неформальные правила v1

- Математика в ядре доступна без `import`.
- Реализация в transpile-to-C использует `math.h`/runtime helper-ы.
- Для embedded/gamedev важен предсказуемый runtime без скрытых тяжелых абстракций.

## 4. Минимальная реализация в компиляторе (выполнено для math core)

1. Parser/AST:

- распознавание builtin-вызовов `atan2`, `root`.

2. Semantic:

- `sin/cos/atan2/root` принимают numeric-типы,
- возвращаемый тип: `Float` (или расширение по правилам текущей типизации).

3. Codegen C:

- `sin` -> `sin(...)`
- `cos` -> `cos(...)`
- `atan2` -> `atan2(...)`
- `root(x, n)` -> `pow(x, 1.0/n)` (или helper),
- добавить `#include <math.h>`.

4. Tests:

- parser + semantic + codegen smoke,
- минимум один e2e сценарий “игровой” математики.

## 5. Отдельные решения после bounded MVP

- Финальный синтаксис Matrix2D (новый тип vs алиас над `List(List(T))`).
- Оптимизированные SIMD/backend-specific lowering для `Vec*`.

Фактический текущий контракт находится в пользовательских страницах
[Математика](../user/math.md) и [Векторы](../user/vectors.md). Предложенная здесь
`Matrix2D` не реализована.

