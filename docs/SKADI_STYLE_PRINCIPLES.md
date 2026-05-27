# Skadi Style Principles

## Status
Accepted baseline for near-term language design decisions.
Date: 2026-05-20

## Primary Goal
Skadi syntax should be read without cognitive strain.

This is not a "shortest syntax" objective.
This is a "smooth reading" objective.

## Core Reading Rule
Prefer forms that read like technical prose, even when they are longer.

Example:
- Prefer `returns` over symbolic `->`.

## Reference Writing Style
The canonical style reference is:
- `examples/example_meteostation.txt`

Any new syntax proposal should be checked against this writing style before adoption.

## Language Positioning
Skadi targets system programming with familiar C-class capabilities, while improving syntax readability.

Practical interpretation:
- keep low-level/system usefulness,
- keep target flexibility through toolchain/targeted builds,
- reduce symbolic noise when it harms readability.

## Canonicality Rule
For each feature, define one canonical way to write it in v1.
Avoid parallel styles for the same operation unless there is a critical technical reason.

## Noise Reduction Rule
Reduce punctuation-heavy forms when a readable keyword form is clearer.

Examples of accepted direction:
- `returns` instead of `->`
- keyword-based constructs where they improve immediate comprehension
- prose loop form: `iterate collection as item` (supported in v1 as canonical style)

## Scope Discipline
Do not expand syntax surface area during v1 unless it directly supports the core user flow.

Core user flow for v1:
- write system-style code,
- read it easily,
- compile predictably.

## Decision Filter for New Syntax
A proposal should be accepted only if it passes all checks:
1. It is easier to read without strain.
2. It does not introduce ambiguity in parsing.
3. It fits the style baseline from `examples/example_meteostation.txt`.
4. It does not create a second competing form for the same feature in v1.

