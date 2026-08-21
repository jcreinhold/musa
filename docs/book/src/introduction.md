# Introduction

Musa is a notation-first music language and workbench. You write a piece as text, and musa compiles it into a score you
can engrave, a performance you can hear, and files other programs can read.

```musa
piece "twinkle" {
    tempo 1/4 = 104;
    meter 4/4;
    key c major;

    score {
        part piano {
            voice melody {
                c4/4 c4/4 g4/4 g4/4 a4/4 a4/4 g4/2
                rest/1
            }
        }
    }
}
```

The source is the only document. The desktop app is a structured editor for that text, not a second place where music
lives: anything the interface can do, the language can say, and anything the language says, the interface shows.

## How this book is organized

The book follows [Diátaxis](https://diataxis.fr). Each section serves one kind of reader need:

- **Tutorials** — lessons for a newcomer. Start with [Getting started](tutorials/getting-started.md).
- **How-to guides** — directions for a task you already know you want: exporting, playback, editor setup, the studio.
- **Explanation** — the ideas the system is built on: the event-track, exact time, layer separation, provenance.
- **Reference** — dry, complete description: the language, the standard library, the CLI, the event track format.

## Where the design lives

This book describes how to use musa. The documents that govern how it is built stay in the repository:

- `docs/rules/` — the constitutional commitments everything else answers to;
- `docs/rules/events/` — the event-track the surface language elaborates into;
- `docs/plan/roadmap.md` — the architecture;
- `docs/rules/desktop/` — the desktop interface specification;
- `docs/plan/prompts/` — the numbered implementation plan.

`docs/README.md` maps the whole tree and says which document wins when two of them disagree. Where those documents and
the code disagree, the code is wrong or the document needs a deliberate repair.
