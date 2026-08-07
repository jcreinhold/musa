/**
 * The only module in the interface that talks to Tauri.
 *
 * Everything else reads the session store. Keeping `invoke`, `listen`, and
 * the file dialogs behind this one file is what lets the same components run
 * in a plain browser against the committed fixtures — which is how the
 * engraving goldens are taken, and how `npm run dev` works without a build of
 * the shell.
 */

import type { CommandDto } from "./generated/CommandDto";
import type { EditDto } from "./generated/EditDto";
import type { ErrorDto } from "./generated/ErrorDto";
import type { ExportTargetDto } from "./generated/ExportTargetDto";
import type { TemplateDto } from "./generated/TemplateDto";
import type { TransportDto } from "./generated/TransportDto";
import type { EditImpact, ProjectSnapshot } from "../state/snapshot";

/** Whether the interface is running inside the desktop shell. */
export function inShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** What the shell emits, and what each event carries. */
export interface Events {
  "musa://snapshot": ProjectSnapshot;
  "musa://position": ProjectSnapshot["playback"];
  "musa://transport": ProjectSnapshot["playback"];
  "musa://command": string;
}

async function core(): Promise<typeof import("@tauri-apps/api/core")> {
  return import("@tauri-apps/api/core");
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  const { invoke } = await core();
  return invoke<T>(command, args);
}

/** True when a rejected promise carries the shell's own failure shape. */
export function isFailure(thrown: unknown): thrown is ErrorDto {
  return (
    typeof thrown === "object" && thrown !== null && "kind" in thrown && "message" in thrown
  );
}

export const bridge = {
  openProject: (path: string) => call<ProjectSnapshot>("open_project", { path }),
  newProject: (template: TemplateDto, path: string | null) =>
    call<ProjectSnapshot>("new_project", { template, path }),
  apply: (command: CommandDto) => call<ProjectSnapshot>("apply", { command }),
  /** What an edit would change, asked before it is made. */
  editImpact: (edit: EditDto) => call<EditImpact>("edit_impact", { edit }),
  transport: (command: TransportDto) => call<ProjectSnapshot>("transport", { command }),
  exportTo: (target: ExportTargetDto, path: string | null) =>
    call<{ path: string }>("export", { request: { target, path } }),
  snapshot: () => call<ProjectSnapshot>("snapshot", {}),

  /** Subscribe to a shell event. Resolves to the unsubscribe function. */
  async on<K extends keyof Events>(
    name: K,
    handle: (payload: Events[K]) => void,
  ): Promise<() => void> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<Events[K]>(name, (event) => handle(event.payload));
  },

  /** Ask the user for a `.musa` file. `null` when they cancel. */
  async askToOpen(): Promise<string | null> {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const chosen = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Musa piece", extensions: ["musa"] }],
    });
    return typeof chosen === "string" ? chosen : null;
  },

  /** Ask the user where to write something. `null` when they cancel. */
  async askToSave(name: string, extension: string): Promise<string | null> {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: name, filters: [{ name: extension, extensions: [extension] }] });
  },
};
