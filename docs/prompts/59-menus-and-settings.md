---
id: 59
slug: menus-and-settings
status: done
depends_on: [21, 23, 55]
phase: 2
---

# Menus and Settings: the Frame's Own Controls

## Task

Make musa's menus read like a desktop application's rather than like a list of function names. Export becomes one verb
with four objects instead of four sibling verbs; the View menu becomes four groups instead of twelve flat items; and the
preferences that were hiding in View move to **Settings** — `⌘,`, in the application menu, where the platform's users
look for them — with a sheet that shows all of them at once.

Nothing new is added to what the application can do. This prompt is entirely about where its existing commands are
found.

## Read

- `docs/interface/03-interaction.md` §3 (the binding table, and the sentence about vim mode this repairs) and §6 (one
  list, three surfaces).
- `docs/interface/01-visual-language.md` §7 (no field boxes; the hairline underline is the whole affordance) and §6 (the
  three animations, none of which is a sheet).
- Prompt 21's `registry.rs` and `menu.rs` — the registry the menu, the palette, and the keyboard sheet are all rendered
  from — and prompt 55's `Preferences` and `ThemeChoice`.

## Design

### The registry says how a command sits in its menu, not only which menu

A flat list of `(id, title, section, accelerator)` cannot say two things a menu needs: that four commands are *one verb
with four objects*, and that a menu has groups. So `CommandDescriptor` gains two fields, each stating one fact:

- `submenu: Option<&'static str>` — the submenu this item sits in.
- `apart: bool` — a separator precedes it, because it begins a new group.

**A submenu's name is a prefix of its members' titles.** `Export MEI` under `Export` shows in the menu as `MEI`, because
`Export ▸ Export MEI` is the same word twice. The palette and the keyboard sheet, which have no nesting to lean on, keep
the whole title. One name in the registry, two renderings by one stated rule — and a test that every submenu really is a
prefix of every member's title, so the rule cannot quietly stop holding.

### The menus

```
musa      About musa · Settings… ⌘, · Services · Hide · Quit
File      New piece ⌘N · Open a piece ⌘O ┊ Save ⌘S ┊ Export ▸ MEI · LilyPond · MusicXML · WAV
Edit      Undo ⌘Z · Redo ⌘⇧Z ┊ Format the source ⌘⇧F ┊ Cut · Copy · Paste · Select All
View      Compose ⌘1 · Sound ⌘2 · Mix ⌘3 · Source ⌘4 ┊ Show the source ⌘' ┊ Zoom out ⌘− · Zoom in ⌘= ·
          Reset zoom ⌘0 ┊ Command palette ⌘K
Help      Keyboard sheet ?
```

The groups are the questions a composer is answering: *which room am I in*, *can I see the text*, *how big is the page*,
*what else is there*. Twelve items with no seams answers none of them, which is the complaint this prompt starts from.

### Settings is where preferences live

`Section::Settings` is a section with no top-level menu. Its one visible representative is **Settings…** (`⌘,`) in the
application menu on macOS, and at the foot of Edit on Windows and Linux, which is where each platform puts it. Choosing
it opens a sheet.

The commands themselves stay in the registry, so the palette still reaches them and their accelerators still hold —
`⌘⌥=` is a preference whether or not a menu item claims it. Their ids move from `view.*` to `settings.*`, because a
`view.vim` that is not in the View menu is a name that lies to the next person who greps for it.

The sheet is the keyboard sheet's shape — a leaf on a scrim, `Esc` closes it — with three rows set as the inspector sets
its own: label at the left, choices at the right, no boxes (§7). Each row is a **set of choices**, not a checkbox:

| Row | Choices |
| --- | --- |
| Theme | System · Light · Dark |
| Text size | Small · Normal · Large · Larger |
| Vim mode | Off · On |

Theme has three because `ThemeChoice` always had three — `chosen: null` is *follow the system* — and the menu's "Switch
theme" could only ever reach two of them. A composer who overrode the theme once had no way back to following the system
short of clearing `localStorage`. Naming the third state is the whole fix.

### What this must not become

- **No preferences window.** One sheet, three rows, no tabs, no sidebar. The moment it needs a second page it is
  carrying something that belongs somewhere else.
- **No document settings in it.** Tempo, key, title, and copyright are the *piece's* and are edited where they are
  printed (prompt 54). A settings sheet that could change the music would be a second editor for the document.
- **No new preference.** Everything in the sheet already existed; this prompt found it a home.
- **No menu behaviour.** A menu item still forwards its id to the webview and nothing else (prompt 21).

## Target

- `apps/musa-desktop/src-tauri`:
  - `registry.rs` — `submenu` and `apart` on `CommandDescriptor`; `Section::Settings`; `view.text.*`, `view.vim`, and
    `view.theme` renamed to `settings.*`; a new `settings.open` bound to `CmdOrCtrl+,`.
  - `menu.rs` — submenus, separators, and the Settings item in the application menu (macOS) or at the foot of Edit
    (elsewhere).
- `apps/musa-desktop/ui`:
  - `src/lib/commands/map.ts` — the renamed ids, the `Settings` group, and `settings.open`.
  - `src/screens/Settings.svelte` — the sheet.
  - `src/lib/session/theme.svelte.ts` — `choose(theme: Theme | null)`, so *follow the system* is reachable.
  - `src/App.svelte` — the sheet's open state, alongside the palette's and the keyboard sheet's.
- Tests:
  - Rust: every submenu is a prefix of its members' titles; every rendered section has at least one item; ids and
    accelerators stay unique.
  - unit: the map and the registry still agree, and every registry section has a frontend group.
  - Playwright: `⌘,` opens the sheet; each row changes the preference it names and the choice it made is marked;
    choosing System after Dark goes back to following the system; `Esc` closes it; the axe scan is clean.

## Check

```sh
cargo nextest run -p musa-desktop
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
```

Commit as `Group the menus and give preferences a home`.

## Found along the way

- **A theme you could not stop overriding.** `ThemeChoice` always had three states — `chosen: null` is *follow the
  system* — but `toggle()` only ever moved between light and dark, so the only control that existed could not reach the
  state the app starts in. `choose(theme: Theme | null)` reaches all three, and the Theme row shows all three.
- **B2 waited for ink rather than for its own ink.** The trial loop waited for *a* `musa:score` mark and then measured
  the one after the keystroke. A background page left over from the previous trial satisfies the first without
  satisfying the second, and the sample came back `NaN` — which failed roughly one full run in two, on a number nobody
  had measured. B8 was repaired this way in prompt 50; B2 now waits the same way.
- **`Settings…` is a named descriptor, not a lookup.** Searching `COMMANDS` for the id needed an `expect`, guarded by a
  test. Naming the descriptor in the registry and putting *that* in the list makes a registry which stopped declaring it
  fail the build instead.
- **Preferences persist, and now say so once.** Text size and vim each had an "outlives the window" test; the theme had
  none. The sheet sets all three together, so one test sets all three, reloads, and reads them back.

## Stop

- No new commands beyond `settings.open`.
- No preferences that do not exist today.
- No custom in-window menu bar: the native menu is the menu, and a second one drawn in the webview would be two menus to
  keep in step.
- No settings that live in the project file. Preferences are the app's; the document is the document's.
