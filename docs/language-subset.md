# Language Subset

Nori V1 intentionally supports a small component syntax subset, not the full JavaScript language. The goal is to grow the compiler gradually while keeping behavior understandable.

## Supported Top-Level Syntax

- `import` declarations.
- `export default function`.
- `export default` expressions.
- `const`, `let`, and `var`.
- `function` declarations.
- `type` and `interface` declarations, which are parsed and stripped.

## Supported Statements

- Blocks.
- `return`.
- Variable declarations.
- Expression statements.
- Basic `if` / `else`.

## Supported Expressions

- Identifiers.
- Numbers, strings, booleans, and `null`.
- Arrays and object literals.
- Function calls.
- Member access.
- Assignments and compound assignments.
- Unary, binary, logical, and ternary expressions.
- Arrow functions.
- Parenthesized expressions.

## Supported Markup

- Elements with opening and closing tags.
- Self-closing elements.
- Fragments.
- Text children.
- Expression containers.
- String attributes.
- Expression attributes.
- Boolean attributes.
- Spread attributes.

## Supported Type Stripping

- Variable annotations.
- Function parameter annotations.
- Function return annotations.
- Simple generic parameter lists.
- `type` declarations.
- `interface` declarations.

## Not Yet Supported

Nori does not aim to parse all JavaScript yet. Known future areas include:

- Classes.
- Full type grammar.
- Complex destructuring patterns.
- Namespaced element names.
- Source maps.
- Full error recovery.
- Markup lowering to runtime calls or DOM code.
