# 09 — Sound and Mix

Status: **governing**.

The Sound and Mix workspaces are projections and structured editors of Musa source. They are not a preset browser over a
hidden Rust catalogue, a generic machine inspector, or a mixer model owned by the webview. The source declaration tree
names instruments, profiles, controls, media, buses, sends, and routes; the host reports only checked projections,
registered support, verified assets, and prepared-plan state.

## 1. Three disclosures

The composer meets sound at three depths:

1. **Compose** states the selected part's effective instrument and performance profile. An inherited edition default is
   labelled inherited and can be written explicitly with one source edit.
2. **Sound** leads with the instrument declaration, profile, declaration origin, exposed musical controls, technique
   support, and asset health. A source-defined graph may be disclosed beneath that contract, but private primitive state
   is read-only and never presented as the instrument's public vocabulary.
3. **Mix** shows part **outputs**, recorded-media sources, buses, sends, routes, and main output. A part output is a
   binding from a score part to a sounding machine; it is not the part and is not called a track.

Imported and bundled declarations open as read-only source. A declaration in the open document opens at its source
token. An interface control never recreates a declaration from its spelling or infers its kind from a display string.

## 2. Authority and editing

Every editable value arrives as an immutable compiler/project fact and returns through a structured edit whose result is
the smallest source replacement. Instrument selection writes the source's instrument/profile sentence. A control gesture
rewrites the exact source token once on commit; intermediate pointer movement is ephemeral UI state and never updates a
prepared plan independently. Send levels follow the same rule.

The interface may format exact quantities for display and map an explicit checked range onto an HTML control. It may not
derive musical defaults, supported controls, units, routes, media duration, asset status, or declaration origin. Missing
facts are a backend defect, not permission for TypeScript semantics.

## 3. State and voice

An invalid current edit leaves Sound and Mix visibly on the last valid plan and states the revision relationship. Asset
facts describe the current verified closure even while musical facts remain last-valid. Each non-ready asset names its
status and the remediation supplied by the project boundary; offline package failures are not collapsed into a generic
loading spinner.

An empty studio is playable and says that the edition instrument routes directly to main. Empty parts, absent media, and
absent buses are distinct empty states. Recorded-media rows state `clip` or `fixed cue`, the source name, asset, fit
policy where applicable, occurrence count, and authored destination. They expose no waveform and manufacture no physical
duration for fixed media.

## 4. Interaction and accessibility

Part, media, and bus groups are keyboard reachable in source order. Native selects and range controls retain visible
labels; every group has an accessible name that distinguishes a part output, media source, or bus. Recompilation keeps
focus on the same semantic name when it survives. At narrow widths the groups become one reading column rather than a
horizontal console. `prefers-reduced-motion` changes nothing because these workspaces use no ambient motion.

No workbench element uses a free-form node canvas, skeuomorphic hardware, hidden mutable preset state, or an editable
generic property grid over private machine internals.
