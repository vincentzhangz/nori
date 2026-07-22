# Language Subset

> See also: [docs index](./README.md) · [Getting started](./getting-started.md) · [Examples](./examples.md)

Nori supports a practical subset of JavaScript and TypeScript syntax. The goal is to grow the compiler gradually while keeping behavior understandable and well-tested.

## Supported Top-Level Syntax

- `import` declarations (default, named, namespace, and side-effect).
- `import type` declarations (parsed and fully erased).
- `export default function`, `export default class`, and `export default` expressions.
- `export { ... }` named exports and `export * from` re-exports (value forms still partly raw-emitted).
- `export type` / `export type { ... }` (parsed and fully erased).
- `const`, `let`, and `var`.
- `function` declarations (with optional parameter/return type annotations).
- Class declarations with supported runtime members and TypeScript erasure.
- `type` and `interface` declarations, which are parsed into a real type AST and stripped.
- Generic type alias parameters (`type Id<T> = T`) are parsed and available to the checker.
- `enum` declarations lowered to a runtime object IIFE (numeric reverse-mapping; string members forward-only).
- `const enum` declarations erased; simple member accesses (`E.A`) are inlined to literal values.
- `module` / `namespace` declarations (parsed; currently erased).

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
- Codegen lowers markup to `h(...)` calls from `@nori/core` (expression children become `() => ...` getters for fine-grained updates).

## Supported TypeScript Type Grammar

Parsed into a real `TSType` AST and erased at emit:

- Keywords (`string`, `number`, `boolean`, `any`, `unknown`, `never`, `void`, `null`, `undefined`, `object`, `symbol`, `bigint`).
- Literals (string / number / boolean / null).
- Type references with type arguments (`Foo<T>`).
- Unions and intersections.
- Array and tuple types.
- Object type literals (properties, methods, index signatures).
- Function types `(a: T) => U`.
- Conditional types `A extends B ? C : D`.
- `infer` in conditional extends clauses.
- Mapped types `{ [K in keyof T]: ... }` (optional `readonly` / `?`).
- Template literal types `` `id-${string}` ``.
- `keyof` / `readonly` type operators.
- Indexed access `T[K]` and `typeof` type queries.
- Parenthesized types.

## Supported Type Stripping / Lowering

- Variable annotations.
- Function parameter and return annotations.
- Simple generic parameter lists on functions and type aliases.
- Expression `as` / `satisfies` erasure.
- Postfix non-null assertion erasure.
- Generic call type argument erasure.
- `type` / `interface` / `import type` / `export type` erasure.
- Regular `enum` → object IIFE (string vs numeric member shapes).
- `const enum` → erased declaration + inlined member values at simple `Enum.Member` sites.

## Type Checker (M1–M8)

- `nori check <file>` always runs semantic analysis + the production checker (M1–M8).
- `CompileOptions.type_check` defaults to **`false`** so `compile` / plugins keep emitting JS without failing on type errors. Set `type_check: true` to surface the same diagnostics from `compile_source`.
- **M1**: annotated declarations (`let x: string = 1`) produce assignability errors.
- **M2**: interface/object structural assignability; excess property checks on object literals.
- **M3**: function parameter annotations checked at calls; annotated return types checked at `return`.
- **M4**: basic generic instantiation for identity aliases (`type Id<T> = T` → `Id<string>` is `string`).
- **M5**: control-flow narrowing for `typeof x === "..."` and truthiness (`if (x)`) in if-branches.
- **M6**: evaluates concrete conditional types `T extends U ? X : Y`; simple `keyof` on object type aliases.
- **M7**: `check_files(paths)` multi-file stub sharing ambient lib globals (`Array` / `Promise` / `String`, see `crates/nori-checker/libs/lib.es5.min.d.ts`).
- **M8**: component prop checking — markup `<Foo bar={...} />` checked against `Foo`'s annotated props object parameter.
- Semantic model builds scopes, symbols, and identifier references.

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

- Full CFG narrowing (discriminated unions beyond typeof/truthiness), mapped/indexed evaluation beyond M6 stubs.
- Full `.d.ts` parsing (lib globals are currently hardcoded to match the vendored stub).
- Generic instantiation beyond simple aliases / concrete conditionals.
- Const enum inlining for computed or non-literal member initializers.
- Full `export` value forms (non-default exports are still partly raw-pass-through).
- Class expressions in expression position (parsed as raw pass-through).
- Namespaced element names.
- Source maps.
- Full statement-level error recovery coverage.
