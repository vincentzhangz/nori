# Language Subset

Nori supports a practical subset of JavaScript and TypeScript syntax. The goal is to grow the compiler gradually while keeping behavior understandable and well-tested.

## Supported Top-Level Syntax

- `import` declarations (default, named, namespace, and side-effect).
- `export default function`, `export default class`, and `export default` expressions.
- `export { ... }` named exports and `export * from` re-exports.
- `const`, `let`, and `var`.
- `function` declarations.
- Class declarations with supported runtime members and TypeScript erasure.
- `type` and `interface` declarations, which are parsed and stripped.

## Supported Statements

- Blocks.
- `return`.
- Variable declarations including destructuring patterns (array and object).
- Expression statements.
- `if` / `else`.
- `try` / `catch` / `finally`.
- `switch` / `case` / `default`.
- `throw`.
- `for`, `for...in`, and `for...of` loops.
- `for await...of` loops.
- `while` and `do...while` loops.
- Labeled statements (`label: stmt`).
- `debugger`.
- `with`.
- Unlabeled `break` and `continue` inside loops.

## Supported Expressions

- Identifiers.
- Numbers (with `1_000` separators), strings, booleans, `null`.
- BigInt literals (`42n`).
- Arrays and object literals with property shorthand and spread.
- `this`.
- `new` expressions (`new Foo()`, `new.target`).
- `delete`, `void`, `typeof` unary operators.
- `super` property access and calls.
- `import.meta` and `import()` dynamic import expressions.
- `yield` / `yield*` expressions.
- Tagged template literals and template literal interpolation.
- Function calls, member access (dot and bracket), index expressions.
- Optional chaining (`?.`) on calls, member access, and index expressions.
- Assignments and compound assignments (`=`, `+=`, `-=`, `*=`, `/=`, `**=`, `&&=`, `||=`, `??=`, `|=`, `&=`, `^=`, `<<=`, `>>=`, `>>>=`).
- Prefix and postfix updates (`++`, `--`).
- Unary (`!`, `-`, `+`, `~`), binary, logical, and ternary expressions.
- Equality operators: `==`, `!=`, `===`, `!==`.
- Relational operators: `<`, `<=`, `>`, `>=`, `in`, `instanceof`.
- Bitwise operators: `|`, `&`, `^`.
- Shift operators: `<<`, `>>`, `>>>`.
- Exponentiation: `**`.
- Nullish coalescing: `??`.
- Arrow functions (expression and block body).
- Sequence expressions (comma operator).
- Parenthesized expressions.
- `await` expressions.

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
- Expression `as` / `satisfies` erasure.
- Postfix non-null assertion erasure.
- Generic call type argument erasure.
- `type` declarations.
- `interface` declarations.

## Supported Classes

- Identifier-named fields with optional runtime initializers.
- Computed class member names (`[expr]`).
- Private fields and methods (`#name`).
- Constructors, methods, static members, and async methods.
- Getter and setter accessors (`get` / `set`).
- Static initialization blocks (`static { ... }`).
- Class-level and member-level decorators (`@Decorator`) — parsed and stripped.
- `extends` in emitted JavaScript.
- Erased class generics and `implements` clauses.
- Erased member/parameter/return annotations and method generics.
- Erased `public`, `private`, `protected`, `readonly`, `abstract`, `declare`, and `override`.
- Constructor parameter properties lowered to runtime assignments.
- Declaration-only members and type-only fields stripped when they have no runtime JavaScript output.

## Not Yet Supported

- Full TypeScript type grammar (union, intersection, conditional, mapped, template literal types, enums, namespaces).
- Class expressions in expression position (parsed as raw pass-through).
- Namespaced element names.
- Source maps.
- Full error recovery.
- Markup lowering to runtime calls or DOM code.
