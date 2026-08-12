# T₂c — source closure corrigendum

**Status: fixed closure target for independent review; governs nothing.** Review 48 accepted T₂b's semantic core and
both mechanism programs after qualification, but found four missing integration definitions. This note adds exactly
those definitions. It imports all unchanged syntax, reduction, rank, sealing, and reducibility clauses from
[47](47-t2b-minimal-source-closure.md).

## 1. Bidirectional `Result` checking

Use the ordinary judgments

```text
Γ ⊢ e ⇒ A    e synthesizes A
Γ ⊢ e ⇐ A    e checks against expected A.
```

Variables, literals with intrinsic domains, applications, projections, annotated definitions, and scrutinees synthesize.
The subsumption rule is exact monomorphic equality:

```text
Γ⊢e⇒A    A=B
────────────── Check-Synth
Γ⊢e⇐B.
```

`Ok` and `Err` are check-only unless enclosed in an explicit type annotation:

```text
Γ⊢e⇐A                         Γ⊢e⇐E
──────────────────── Ok-Check ──────────────────── Err-Check
Γ⊢Ok(e)⇐Result A E            Γ⊢Err(e)⇐Result A E.
```

Expected types propagate into:

- the body of an explicitly typed `let` or function result;
- an application argument from the synthesized function domain;
- a nominal constructor field from its owner-qualified constructor signature;
- product, `Some`, list, `Ok`, and `Err` children from their expected structural type; and
- every match arm from the match expression's expected result type.

A result match first synthesizes its scrutinee as `Result A E`; it checks its `Ok` and `Err` arms under `x:A` and `y:E`
against one expected result, or synthesizes the first arm and checks the other against that exact type. A bare
`Ok(0)`/`Err(0)` in a synthesis position is rejected as ambiguous. A general type annotation can make it synthesize:

```text
(Ok(0) : Result Nat Text) ⇒ Result Nat Text.
```

### Lemma 1.1 — bidirectional checking is decidable and sound

For finite syntax and a finite functional environment, synthesis returns at most one type, checking returns a boolean or
finite diagnostic, and every accepted bidirectional judgment has the corresponding declarative T₂b typing judgment.

*Proof.* Structural induction on syntax. Every synthesis case performs functional lookup or recursively synthesizes a
fixed subterm and applies exact decidable type equality. Every checking case is determined by the expected outer type.
`Ok` selects its `A` component and `Err` its `E` component, so neither searches for the absent type parameter. Result
matching obtains both parameters from one synthesized scrutinee. Erasing `⇒/⇐` from each rule yields T₂b's declarative
introduction, elimination, or exact conversion derivation. ∎

The operational semantics and reducibility proof are unchanged; bidirectionality is elaboration, not reduction.

## 2. Complete project dependency judgment

Per-structure checking remains T₂b Lemma 3.1. Project acceptance additionally uses:

```text
BuildClosure ⊢ Project
  ⇒ Σ_public ; Δ_data ; Γ_core ; definitions.
```

The algorithm:

1. resolves the finite immutable local/bundled/pinned build closure and rejects import/module cycles;
2. collects every top-level and structure member header, nominal stamp, and generated instance header;
3. checks each ordinary structure locally and flattens every private/public value body to one owner-qualified core
   definition;
4. records an edge `x→y` for every free reference from definition `x` to definition `y`, including qualified
   cross-structure paths, imported values, and generated instance dependencies;
5. rejects any cycle in the **one complete definition graph**; and
6. returns definitions in the canonical topological order, tie-broken by owner-canonical definition path.

The graph includes ordinary non-structure definitions as well as structure members. The project

```text
structure A : S { let x : Nat = B.x; }
structure B : S { let x : Nat = A.x; }
```

has the cycle `A.x→B.x→A.x` and is rejected even though both local sibling graphs are empty.

### Lemma 2.1 — accepted projects flatten to non-recursive lets

If the project judgment accepts, its returned definition sequence elaborates to one finite well-typed nest of
non-recursive owner-qualified `let` bindings.

*Proof.* Header collection gives one type to every graph vertex. Local checking plus T₂b sealing proves each flattened
body at that type under the collected environment. Acyclicity supplies a topological order in which every free
definition dependency precedes its consumer. Induct over that order, applying weakening and the governing substitution
lemma. The resulting finite nesting has no forward free name and no recursive binding. ∎

This is the governing total core's existing project argument extended over nominal structure members, not a second
module-specific evaluation rule.

## 3. Canonical data for `Text` and `Result`

### 3.1 `Text`

One `Text` value is a finite Unicode scalar sequence. Its canonical bytes are:

```text
frame(
  text_type_id,
  text_schema_version,
  scalar_count,
  utf8_byte_count,
  utf8_bytes,
).
```

Valid UTF-8 encodes a Unicode scalar sequence uniquely. The decoder validates both counts and rejects an invalid,
overlong, surrogate, or trailing encoding. Text equality and canonical-byte equality are exact scalar-sequence equality.
No Unicode normalization is performed or promised. A future normalizer would be a separately versioned compiler-owned
primitive; T₂c does not claim a package can implement one with `text_equal` alone.

### 3.2 `Result`

Given `CanonicalData A` and `CanonicalData E`, the structural schema of `Result A E` contains the exact ordered child
schema descriptors. Its values encode as:

```text
frame(result_type_id,result_schema_version,schema(A),schema(E),OkTag, frame(enc_A(a)))
frame(result_type_id,result_schema_version,schema(A),schema(E),ErrTag,frame(enc_E(e))).
```

`OkTag≠ErrTag`. Result equality requires the same tag and child equality in the corresponding admitted child schema.

### Lemma 3.1 — the new encodings are exact

Canonical byte equality is equivalent to admitted value equality for `Text` and for `Result A E` whenever the child
canonical-data contracts hold.

*Proof.* Unique valid UTF-8 gives the `Text` result after the framed header/counts are decoded. For `Result`, unique
framing first determines both child schemas and the distinct tag, then reduces byte equality to exactly one child's
canonical-data equivalence. The converse in each case is deterministic re-encoding of equal components. ∎

T₂b Lemma 5.1 can therefore recurse through every field type T₂c admits.

## 4. Owner package identity for every build-closure class

Musa has no package registry or version solver. The finite build closure has one exact owner table. Define:

```text
OwnerPackageRef ::=
  LocalRoot(stable_project_id, canonical_package_name)
| Bundled(distribution_identity, canonical_package_name)
| PinnedGit(canonical_remote_url,
            commit_algorithm, full_commit_object_id,
            verified_tree_algorithm, verified_tree_digest,
            canonical_package_subdirectory).
```

- `stable_project_id` is a source-controlled opaque id in the root `musa.toml`, generated once and retained when the
  project moves. Copying a project intentionally copies its nominal owner; forking its public type identities requires
  changing the id and declaring a migration.
- `distribution_identity` is the exact compiler/standard-library distribution version plus the canonical bundled package
  manifest identity.
- `PinnedGit` is the exact lock entry after fetch verification. Import aliases are absent. Two URLs or commits are
  distinct owners even if their current trees happen to match.

The owner table maps each reference to one canonical finite **owner descriptor**. For `LocalRoot`, equality of the
source-controlled stable id and package name deliberately means the same nominal owner across ordinary source edits;
copying both is an explicit identity-preserving fork as stated above. The descriptor does not contain the whole changing
source tree. Bundled and pinned references contain immutable distribution/commit identities; the corresponding exact
manifest or fetched tree is verified against those references when the build closure is loaded. A digest may locate a
descriptor or immutable tree, but a digest collision never identifies two unequal complete references or verified
manifests. This is a build-closure table, not a network registry.

`StampKey` remains:

```text
(nominal_format_version,
 OwnerPackageRef,
 owner_module_path,
 owner_structure_path,
 member_name,
 member_schema_version).
```

Every component is exact framed canonical data. Ordinary local edits therefore do not change every nominal stamp. A
public nominal schema edit changes `member_schema_version`. Renaming/moving the owner path is an identity change unless
an explicit migration preserves an old external key.

### Lemma 4.1 — owner identity is total and alias-independent

Every nominal declaration in an accepted current or planned build closure has exactly one `OwnerPackageRef`, and two
source import aliases of one declaration yield the same `StampKey`.

*Proof.* Closure resolution classifies the root as `LocalRoot`, standard packages as `Bundled`, and every remote lock
node as `PinnedGit`; the constructors are disjoint. Within one class, the functional owner table and exact conflict
check give one descriptor. Name resolution replaces an import alias by the owner package reference and canonical module
path before stamp construction, so the alias is absent from the key. ∎

## 5. Corrected representation-opacity statement

T₂b Lemma 4.1 is read as follows: a client may bind, pass, return, or discard a value of opaque type `M.T`. It cannot
name a private constructor, distinguish constructor cases, or access constructor fields. Target validation retains the
private signature in `Δ_data`; source lookup omits it from `Σ_public`.

This is representation opacity, not linearity and not an inability to use the value.

## 6. Owner-qualified complete probes

In the core notation below a tag is qualified by its nominal stamp. T₂b's W bodies are exactly:

```text
written = λp. WrittenValue_{W.Written}(p)
motion = λi. MotionValue_{W.Motion}(i)

observe_written = λw.
  match w with { WrittenValue_{W.Written}(p) => p }

observe_motion = λm.
  match m with { MotionValue_{W.Motion}(i) => i }.
```

For K, systematically qualify the tags:

```text
ContextValue_{K.Context}
SvaraValue_{K.SvaraIntent}
LearnedGesture_{K.GamakaIntent}
BoundUnit_{K.Phrase}
Trajectory_{K.Gesture}
EmptyContour_{K.Error}.
```

No short-tag lookup is part of the core. A surface elaborator may omit an owner only when an expected constructor field
or synthesized scrutinee type determines exactly one tag; ambiguity is a diagnostic.

Extend K's signature and body with:

```text
let same_name : Text → Text → Bool;
let summarize : Result Phrase Error → Text;

same_name = λleft. λright. text_equal(left,right);

summarize = λresult.
  match result with {
    Ok(phrase) => "accepted";
    Err(error) =>
      match error with {
        EmptyContour_{K.Error}(name) => name;
      };
  }.
```

The literal is checked as `Text`; the result scrutinee synthesizes `Result K.Phrase K.Error`; both arms check as `Text`.
Thus the complete probes exercise a text literal, text equality, result introduction, and result elimination. They
remain mechanism probes rather than claims about Karnatak music.

## 7. Closed theorem

### Theorem 7.1 — T₂c source closure

Assume the governing foreign-operation, contextual-`Music`, deterministic resource, and immutable build-closure
contracts. For every project accepted by the T₂c project judgment:

1. checking and elaboration are decidable and functional;
2. the flattened core program is well typed and finite;
3. private constructors are representation-opaque to clients;
4. evaluation is deterministic, preserves types, makes progress, and strongly normalizes;
5. every nominal value has exact versioned canonical identity; and
6. importing one locked owner through different aliases does not change nominal type identity.

*Proof.* Lemma 1.1 supplies decidable sound bidirectional `Result` checking. Lemma 2.1 supplies the finite acyclic
well-typed core program. T₂b Lemma 4.1 with §5 supplies opacity. T₂b Theorem 6.1 supplies the extended core metatheory,
now using the whole-project premise rather than local Lemma 3.1. T₂b Lemma 5.1 is discharged for every field grammar by
Lemma 3.1 and the existing compiler-owned canonical-data contracts. Lemma 4.1 supplies total alias-independent owner
stamps. The deterministic resource meter may reject before evaluation; for an accepted budget, the unique normal form is
reached. ∎

## 8. Decision if the proof closes

Proof closure would justify an implementation prompt for this exact source fragment. It would not justify the richer
`InContext` template, aliases, recursive data, dependent indices, first-class modules, worlds, or theta-links. Those
remain behind concrete failed-program evidence.

The implementation spike must still replace one toy W operation and one toy K operation with real algorithms before the
nominal fragment graduates. Cultural adequacy is empirical and participatory; Theorem 7.1 proves none of it.
