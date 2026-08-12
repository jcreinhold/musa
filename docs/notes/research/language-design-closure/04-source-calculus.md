# The proposed source calculus

**Purpose:** define the smallest source language selected by the case studies.

This is a research draft, not yet a governing specification. It states the language precisely enough to write examples
and prove its basic properties. It does not specify package version solving, persistent compiled files, or a cache.

## 1. The design in one page

The source language is a small, total, call-by-value language. *Total* means that every accepted source program
finishes. *Call by value* means that a function receives values, not unevaluated expressions.

The language has:

- ordinary finite data;
- named, non-recursive functions;
- finite list folds;
- structures and signatures for hiding constructors;
- an abstract `Music` type for finite score-making recipes; and
- a small set of checked compiler operations.

It does not have general recursion, effects, streams, dependent types, or first-class modules. Live audio runs in the
process stage, outside this language.

Here is a complete example of user-defined data:

```text
data Function {
  Tonic;
  Predominant;
  Dominant;
}

let follows(left: Function, right: Function): Bool =
  match left {
    Tonic =>
      match right {
        Predominant => true;
        _ => false;
      };
    Predominant =>
      match right {
        Dominant => true;
        _ => false;
      };
    Dominant =>
      match right {
        Tonic => true;
        _ => false;
      };
  };
```

The program finishes because `follows` has no way to call itself and every match covers every input.

## 2. Types

Let `mu` range over build-local names of user-defined data types. The core type grammar is:

```text
base ::= Unit | Bool | Nat | Ratio | Text | Duration | Music

type ::= base
       | mu
       | (type_1, ..., type_n)
       | type_1 -> type_2
       | Option<type>
       | List<type>
       | Result<type, type>
       | bridge-type
```

A `bridge-type` is a compiler-owned type used at one stage boundary, such as written `Pitch` or `NoteName`. Section 8
limits these types. A theory package may not add a bridge type by asking the compiler for another special case. It must
use ordinary user-defined data unless a stage boundary requires more.

`Unit`, `Bool`, and `Nat` have their usual meanings. `Nat` contains the non-negative integers that fit the
implementation's stated bound. Checked operations report overflow instead of wrapping.

`Ratio` is an exact reduced fraction within the implementation's stated integer bounds. Its denominator is positive.
Arithmetic returns `Result<Ratio, ArithmeticError>` when the exact answer is out of range or a division uses zero. No
floating-point number enters logical musical time.

`Text` is a finite sequence of Unicode scalar values. Equality compares that sequence exactly. The language does not
silently normalize spelling, case, or Unicode form. The initial operations are equality and concatenation.

`Duration` is a non-negative rational duration accepted by the temporal kernel. It is a bridge to exact musical time,
not a meter, beat, tempo, or performed number of seconds.

`Result<A, E>` contains either `Ok(a)` for `a: A` or `Err(e)` for `e: E`. It reports an expected failure as data. It is
not an exception.

`Music` is defined in Section 9. It is abstract: source programs can combine music through named compiler operations,
but cannot inspect its private representation.

## 3. Finite user-defined data

A data declaration has this form:

```text
data Name {
  Constructor_1(field_1: data-type, ..., field_m: data-type);
  ...
  Constructor_n(field_1: data-type, ..., field_k: data-type);
}
```

The repeated dots above describe the grammar; they are not a paper program. A concrete program must list every
constructor and field.

A `data-type` may contain:

- a base data type other than `Music`;
- an earlier user-defined data type;
- a product;
- `Option`;
- `List`; or
- `Result`.

It may not contain a function, `Music`, a process, a timeline, or itself. This restriction makes each user-defined value
finite and keeps its equality and printing rules simple.

Data declarations form a dependency graph. Draw an arrow from `A` to `B` when a field of `B` contains `A`. The compiler
rejects a cycle. It then gives each type a rank:

```text
rank(B) = 1                                      if B has no nominal field
rank(B) = 1 + max(rank(A))                       otherwise
```

where `A` ranges over the user-defined types found in fields of `B`. Rank zero is reserved for types containing no
user-defined type. This choice matters in the termination proof.

## 4. Terms, values, and evaluation order

The core terms are:

```text
term ::= variable
       | literal
       | (term_1, ..., term_n)
       | lambda(variable: type) => term
       | term(term)
       | let variable = term; term
       | constructor(term_1, ..., term_n)
       | None | Some(term)
       | [] | term :: term
       | Ok(term) | Err(term)
       | match term { arm_1 ... arm_n }
       | fold_list(list, initial, step)
       | primitive(term_1, ..., term_n)
       | annotation(term, type)
```

Source syntax may offer named multi-argument functions. Elaboration turns them into nested one-argument core functions.
Source programs do not contain anonymous functions; the compiler creates the needed core functions for named definitions
and folds.

Values are literals, products of values, closed functions, constructors whose fields are values, and the value forms of
`Option`, `List`, and `Result`.

Evaluation is left to right. In a call `f(a)`, the language first evaluates `f`, then `a`, then the function body. In a
constructor, product, or primitive call, it evaluates fields from left to right. This order makes diagnostics and
resource charges repeatable.

The main reduction rules are:

```text
(lambda(x: A) => body)(value)  -->  body[value / x]

let x = value; body            -->  body[value / x]

match C_i(values) {
  C_i(names) => body_i;
  other arms
}                              -->  body_i[values / names]

match Ok(value) {
  Ok(x)  => good;
  Err(e) => bad;
}                              -->  good[value / x]
```

`fold_list` reduces one list cell at a time. It may use a named step function, but that function cannot call itself. No
other rule can create an unbounded reduction.

## 5. Patterns and exhaustive matching

The first language uses flat patterns:

```text
pattern ::= _
          | variable
          | literal
          | (variable_1, ..., variable_n)
          | Constructor(variable_1, ..., variable_n)
          | None | Some(variable)
          | [] | variable :: variable
          | Ok(variable) | Err(variable)
```

A pattern may not nest another constructor pattern, repeat a variable, or carry a guard. Programs can use a second match
when they need a deeper test. This keeps coverage checking exact and easy to explain.

A match is exhaustive when one of these tests succeeds:

- it has `_` or a bare variable arm;
- a `Bool` match covers `true` and `false`;
- an `Option` match covers `None` and `Some`;
- a list match covers `[]` and `head :: tail`;
- a `Result` match covers `Ok` and `Err`; or
- a user-data match covers every constructor visible inside the defining structure.

Outside a sealed structure, clients cannot name private constructors. They must use the operations exported by the
structure. A match on an abstract type is therefore rejected.

For `Nat`, `Ratio`, `Text`, functions, products, `Music`, and opaque bridge types, a match needs a catch-all arm unless
a compiler-owned rule proves coverage.

## 6. Bidirectional type checking

The checker uses two judgments:

```text
Gamma |- term => A        term produces type A
Gamma |- term <= A        term is checked against expected type A
```

`Gamma` maps names to types. Synthesis (`=>`) works when the term carries enough information to find its type. Checking
(`<=`) works when a surrounding declaration or call supplies the expected type.

Representative synthesis rules are:

```text
Gamma(x) = A
----------------
Gamma |- x => A

Gamma |- f => A -> B    Gamma |- argument <= A
-----------------------------------------------
Gamma |- f(argument) => B

constructor C: A_1 * ... * A_n -> mu
Gamma |- e_i <= A_i for every i
-------------------------------------
Gamma |- C(e_1, ..., e_n) => mu
```

Representative checking rules are:

```text
Gamma |- e <= A
---------------------------
Gamma |- Some(e) <= Option<A>

Gamma |- e <= A
------------------------------
Gamma |- Ok(e) <= Result<A, E>

Gamma |- e <= E
------------------------------
Gamma |- Err(e) <= Result<A, E>
```

`None`, `[]`, `Ok`, and `Err` do not synthesize a unique type. They need an expected type or an explicit annotation. For
example:

```text
let answer: Result<Nat, Text> = Ok(3);
```

is accepted, while a free-standing `Ok(3)` is not.

A match first synthesizes or checks its subject. Each pattern extends `Gamma` with its bound variables. If the match has
an expected result type, every arm is checked against it. Otherwise the first arm synthesizes a type and every later arm
is checked against that same type.

The first matching arm in source order is chosen. A catch-all may therefore follow specific cases without making the
step ambiguous.

No subtype relation hides a mismatch. The only implicit conversion is the one already named by a compiler bridge and
recorded by the stage pass that uses it.

## 7. Definitions, structures, and signatures

Top-level value definitions form another finite dependency graph. Draw an arrow from `x` to `y` when the body of `y`
refers to `x`. This graph includes all definitions in the resolved build, not merely siblings in one structure. The
compiler rejects a cycle and checks definitions in dependency order.

A signature exposes values and may name abstract data:

```text
signature BOX {
  type Item;
  let make: Nat -> Item;
  let size: Item -> Nat;
}
```

A matching structure defines one finite data type for each abstract type and defines every exported value:

```text
structure Box: BOX {
  data Item {
    Packed(value: Nat);
  }

  let make(value: Nat): Item = Packed(value);

  let size(item: Item): Nat =
    match item {
      Packed(value) => value;
    };
}
```

Outside `Box`, the type `Box.Item` is visible but `Packed` is not. The checker keeps two environments:

- the public environment, which clients see; and
- the private data environment, which compiled bodies use to check and run constructor operations.

Sealing removes constructors from the public environment. It does not erase the private information needed to run `make`
and `size`.

The first design has no type aliases, manifest type equations, first-class modules, nested structures, or extra public
data members. A structure may have private helper values. This surface is enough to define a concept, hide its
representation, and export total operations.

## 8. Compiler operations

A compiler operation has a closed type and an implementation supplied by one stage adapter. The adapter must satisfy
this contract:

1. it is defined on every well-typed input;
2. it is pure and deterministic;
3. it returns in finite time;
4. it returns a value of its declared type; and
5. its resource cost is charged by the evaluator.

Ordinary compiler operations are first order: their argument and result types contain no function type. `fold_list` and
the other finite folds are language reduction rules, not foreign operations.

The current language has one higher-order music transform, `map_note_pitches`. It remains only under its existing
stronger contract: the input `Music` recipe has a finite occurrence bound; the transform visits only its documented
finite pitch positions; it applies the supplied total source function once at each visited position; and its proof shows
that this traversal preserves good values and ends. No open registry may add another higher-order compiler operation.
This narrow exception preserves the current language without letting an operation hide an unbounded callback loop.

Expected failures return `Result`. A compiler panic is an implementation bug, not a language outcome.

The small general numeric surface is:

```text
data Ordering { Less; Equal; Greater; }

data ArithmeticError {
  DivisionByZero;
  OutOfRange;
}

ratio_add: Ratio -> Ratio -> Result<Ratio, ArithmeticError>
ratio_sub: Ratio -> Ratio -> Result<Ratio, ArithmeticError>
ratio_mul: Ratio -> Ratio -> Result<Ratio, ArithmeticError>
ratio_div: Ratio -> Ratio -> Result<Ratio, ArithmeticError>
ratio_neg: Ratio -> Result<Ratio, ArithmeticError>
ratio_compare: Ratio -> Ratio -> Ordering
nat_add: Nat -> Nat -> Result<Nat, ArithmeticError>
```

An implementation may use wider integers while calculating, then reduce the fraction and check the public bound. It may
not round.

Written pitch, note spelling, and duration constructors remain compiler bridges for now because notation consumes them
directly. Key, scale, chord, scale degree, Roman numeral, voicing, pitch-class set, and row are ordinary library
concepts. If later work needs a faster representation, it may add a private library implementation without changing the
language.

## 9. Denotational meaning

The operational rules say how a program runs. A denotation says what result the program stands for, without listing its
reduction steps.

Write `[[A]]` for the set described by type `A`:

- `[[Unit]]` has one value;
- `[[Bool]]`, `[[Nat]]`, `[[Ratio]]`, `[[Text]]`, and `[[Duration]]` are their finite canonical implementation values;
- `[[A * B]]` is the set of pairs from `[[A]]` and `[[B]]`;
- `[[Option<A>]]` contains `None` and `Some(a)` for `a` in `[[A]]`;
- `[[List<A>]]` is the set of finite lists over `[[A]]`;
- `[[Result<A, E>]]` contains `Ok(a)` and `Err(e)`;
- `[[A -> B]]` is the set of total functions from `[[A]]` to `[[B]]`; and
- a user-defined type is the tagged union of the products named by its constructors.

The acyclic rank order defines user types one at a time, so this last clause does not refer to itself. Abstract types
have the same private set inside their structure; clients can use the set only through exported functions.

`[[Music]]` is the set of finite score recipes whose application behavior satisfies Section 10. A recipe denotes a total
function from an explicit notation context to either a finite kernel fragment or a stated error. Source code cannot call
that function directly; the stage boundary does.

A well-typed term denotes a total function from the values of its free variables to the value of its result type.
Products, constructors, matches, and folds have their usual set meanings. A compiler operation denotes the total
function required by its contract. The proof later shows that evaluation reaches the value denoted by the term.

This semantics does not claim that every mathematical function can be written in Musa, or that equality of two source
functions is decidable.

## 10. What `Music` means

`Music` is a finite recipe for producing score facts. It is not a score, a performance, an analysis, an audio graph, or
a universal model of music.

The source evaluator treats `Music` as an abstract value. A recipe may contain finite choices already made by the
program, named notation requests, and references to source spans. It cannot read a hidden key, tuning, tempo, or global
current score.

After source evaluation, the compiler applies a recipe to an explicit notation context:

```text
instantiate:
  Music * NotationContext
  -> Result<KernelFragment<ScoreFact>, InstantiationError>
```

`NotationContext` contains only stage facts needed by notation: for example, the selected staff, clef policy, and
capabilities of the target notation adapter. A theory key, raga, ensemble tuning, or harmonic interpretation is a normal
source value passed to the package operation that built the `Music`. It is not smuggled into `NotationContext`.

`KernelFragment<ScoreFact>` is finite but may still have named open ports or placement requests. Closing checks those
requests:

```text
close:
  KernelFragment<ScoreFact>
  -> Result<Term<ScoreFact>, ClosureError>
```

On success, `Term<ScoreFact>` is a closed, well-typed term in the accepted temporal kernel. On failure, the result names
the unfilled request or invalid placement. No partial kernel term proceeds to rendering.

This definition makes `Music` contextual without adding dependent types or hidden state. It also leaves room for a
performance-led package to produce a gesture without producing `Music` at all.

## 11. Build-local type identity

One build begins with a finite, resolved package graph. Every resolved package node, logical module path, and data
declaration path receives a fresh type name. Repeated imports of the same resolved package node share that name.
Distinct nodes do not, even when their source happens to match.

Type equality during the build compares these fresh names. Public constructor hiding and module checking use the same
names. Renaming all fresh names in a consistent way does not change evaluation.

The language promises nothing about reusing a fresh name in another build. It also promises no stable compiled ABI,
persisted function value, or compiled artifact cache. Exact Git source packages remain compatible with this rule: the
resolver supplies one finite graph, then checking assigns fresh names.

## 12. Resource charging

The mathematical reduction relation does not need a resource meter. The compiler evaluator does. Its state contains a
remaining budget and an ordered diagnostic log.

Each syntax form and compiler operation has a fixed, versioned charging rule. Evaluation charges before it performs that
event. Given the same checked core term, initial budget, and compiler version, it produces the same result, remaining
budget, and diagnostics.

If the budget runs out, compilation reports resource exhaustion and publishes no source value. If evaluation succeeds,
its value is exactly the value from the unmetered semantics.

This effort does not specify cache hits. A later cache design must replay the same logical charges and diagnostics as a
miss; it may not change which programs the compiler accepts.

## 13. What must be proved

The proof must establish:

- resolution and type checking decide an answer;
- substitution keeps types;
- evaluation keeps types and does not get stuck;
- evaluation is deterministic and ends;
- sealing prevents clients from constructing an abstract value directly;
- accepted old expressions keep their old meaning;
- closing `Music` either reports a stated error or yields a finite, typed temporal term; and
- the recorded stage conversions compose.

Those claims appear in the proof files after the examples test this design.
