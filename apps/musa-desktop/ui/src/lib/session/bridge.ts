/**
 * The only module in the interface that talks to Tauri.
 *
 * Everything else reads the session store. Keeping `invoke`, `listen`, and
 * the file dialogs behind this one file is what lets the same components run
 * in a plain browser against the committed fixtures — which is how the
 * engraving goldens are taken, and how `pnpm run dev` works without a build of
 * the shell.
 */

import type { CommandDto } from "./generated/CommandDto";
import type { BarlinePreviewDto } from "./generated/BarlinePreviewDto";
import type { PlacementPlanDto } from "./generated/PlacementPlanDto";
import type { PlacementReportDto } from "./generated/PlacementReportDto";
import type { EditDto } from "./generated/EditDto";
import type { ErrorDto } from "./generated/ErrorDto";
import type { DawProfileDto } from "./generated/DawProfileDto";
import type { DawReportDto } from "./generated/DawReportDto";
import type { ExportTargetDto } from "./generated/ExportTargetDto";
import type { GroupEditDto } from "./generated/GroupEditDto";
import type { GroupEditPlanDto } from "./generated/GroupEditPlanDto";
import type { ReviewActionDto } from "./generated/ReviewActionDto";
import type { ReviewAuditionDto } from "./generated/ReviewAuditionDto";
import type { ReviewFactsDto } from "./generated/ReviewFactsDto";
import type { TemplateDto } from "./generated/TemplateDto";
import type { TransportDto } from "./generated/TransportDto";
import type { AnalysisFacts, EditImpact, LibraryDocument, ProjectSnapshot } from "../state/snapshot";

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
  return typeof thrown === "object" && thrown !== null && "kind" in thrown && "message" in thrown;
}

export const bridge = {
  openProject: (path: string) => call<ProjectSnapshot>("open_project", { path }),
  newProject: (template: TemplateDto, path: string | null) => call<ProjectSnapshot>("new_project", { template, path }),
  /** Turn to another file of the project already open. */
  showPiece: (file: string) => call<ProjectSnapshot>("show_piece", { file }),
  /** Write every piece of it that has unsaved edits. */
  saveAll: () => call<ProjectSnapshot>("save_all", {}),
  apply: (command: CommandDto) => call<ProjectSnapshot>("apply", { command }),
  /** What an edit would change, asked before it is made. */
  editImpact: (edit: EditDto) => call<EditImpact>("edit_impact", { edit }),
  /** The exact proved barline transaction, asked before it is applied. */
  barlineRewrite: () => call<BarlinePreviewDto>("barline_rewrite", {}),
  /**
   * What one musical command would do to a selection, asked before it is
   * done. The reply is the transaction, named by an identity that accepting
   * it consumes.
   */
  groupEditPlan: (edit: GroupEditDto) => call<GroupEditPlanDto>("group_edit_plan", { edit }),
  /**
   * Open the take just captured for review. Nothing is written: the reply is
   * a reading of what was played against this piece's meter, tempo, and key.
   */
  reviewBegin: () => call<ReviewFactsDto>("review_begin", {}),
  /** The current review, or `null` when nothing is under review. */
  reviewRead: () => call<ReviewFactsDto | null>("review_read", {}),
  /** One gesture on the review; the reply is the reading it produced. */
  reviewAct: (action: ReviewActionDto) => call<ReviewFactsDto>("review_act", { action }),
  /** Take back the last review decision. */
  reviewUndo: () => call<ReviewFactsDto>("review_undo", {}),
  /** Hear the take as played, or the notation as written. */
  reviewAudition: (mode: ReviewAuditionDto) => call<ReviewFactsDto>("review_audition", { mode }),
  /** Accept the reading. Placing it into the score is a separate step. */
  reviewAccept: () => call<ReviewFactsDto>("review_accept", {}),
  /** Close the review and drop the take with it. */
  reviewDiscard: () => call<null>("review_discard", {}),
  /**
   * What keeping the accepted phrase would write, before it is written.
   * `voices` names one destination voice per line; empty asks for the names
   * the project would offer.
   */
  reviewPlacementPlan: (voices: string[]) => call<PlacementPlanDto>("review_placement_plan", { voices }),
  /** Keep the accepted phrase: one revision, or none. */
  reviewPlace: (voices: string[]) => call<PlacementReportDto>("review_place", { voices }),
  transport: (command: TransportDto) => call<ProjectSnapshot>("transport", { command }),
  exportTo: (target: ExportTargetDto, path: string | null) =>
    call<{ path: string }>("export", { request: { target, path } }),
  /**
   * Package one whole bundle for a workstation, into a directory.
   *
   * The reply is the project's own report of what it wrote. It arrives at
   * the end because the directory is installed whole or not at all: there is
   * no half-written bundle for this side to show progress through.
   */
  exportDawBundle: (profile: DawProfileDto, path: string, replace: boolean) =>
    call<DawReportDto>("export_daw_bundle", { request: { profile, path, replace } }),
  snapshot: () => call<ProjectSnapshot>("snapshot", {}),
  /**
   * Say where in the score the composer is, so a connected keyboard auditions
   * with that part's sound. A keyboard is always listened to; there is no
   * armed writing state for it to enter.
   */
  auditionAt: (caret: string | null) => call<ProjectSnapshot>("audition_at", { caret }),
  selectMidiInput: (id: string) => call<ProjectSnapshot>("select_midi_input", { id }),
  startMidiCapture: (caret: string | null) => call<ProjectSnapshot>("start_midi_capture", { caret }),
  stopMidiCapture: () => call<ProjectSnapshot>("stop_midi_capture", {}),
  keepRecentMidi: (caret: string | null) => call<ProjectSnapshot>("keep_recent_midi", { caret }),
  clearRecentMidi: () => call<ProjectSnapshot>("clear_recent_midi", {}),
  setRecentMidi: (enabled: boolean) => call<ProjectSnapshot>("set_recent_midi", { enabled }),
  /**
   * Read the last valid score and report what one analysis saw. Asked for,
   * never volunteered (`08-elaboration.md` §5).
   */
  analyze: (kind: string) => call<AnalysisFacts>("analyze", { kind }),
  /**
   * Open a bundled library module. `start`/`end` are the handle the snapshot
   * handed out beside the URI, passed back untouched — they index a document
   * this side has never seen.
   */
  libraryDocument: (uri: string, start: number | null, end: number | null) =>
    call<LibraryDocument>("library_document", { uri, start, end }),

  /** Subscribe to a shell event. Resolves to the unsubscribe function. */
  async on<K extends keyof Events>(name: K, handle: (payload: Events[K]) => void): Promise<() => void> {
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

  /** Ask the user for a folder of pieces. `null` when they cancel. */
  async askToOpenProject(): Promise<string | null> {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const chosen = await open({ multiple: false, directory: true });
    return typeof chosen === "string" ? chosen : null;
  },

  /** Ask the user where to put a folder. `null` when they cancel. */
  async askWhereToPut(name: string): Promise<string | null> {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath: name });
  },

  /** Ask the user where to write something. `null` when they cancel. */
  async askToSave(name: string, extension: string): Promise<string | null> {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({
      defaultPath: name,
      filters: [{ name: extension, extensions: [extension] }],
    });
  },
};
