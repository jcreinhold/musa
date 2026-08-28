/**
 * How a workstation bundle is *arranged* for reading, once the project has
 * written it.
 *
 * Nothing here computes a manifest. The project decided which files a bundle
 * holds, what they hash to, and what the target formats could not carry; this
 * module groups that answer into the order a person reads it in and names the
 * suggested folder before there is an answer at all.
 */

import type { DawFileDto } from "../session/generated/DawFileDto";
import type { DawLossDto } from "../session/generated/DawLossDto";
import type { DawProfileDto } from "../session/generated/DawProfileDto";
import type { DawReportDto } from "../session/generated/DawReportDto";

/** The workstations a bundle can be packaged for, and what to call them. */
export const PROFILES: { value: DawProfileDto; label: string; note: string }[] = [
  {
    value: "logic",
    label: "Logic Pro",
    note: "Notation and production: MusicXML, both MIDI documents, and the audio.",
  },
  {
    value: "garageBand",
    label: "GarageBand",
    note: "The MIDI and audio forms GarageBand documents. No notation file.",
  },
];

/** What to call the folder, before anyone has said where to put it. */
export function folder(piece: string | null): string {
  const stem = (piece ?? "piece").replace(/\.musa$/, "").trim();
  return `${stem === "" ? "piece" : stem} for a workstation`;
}

/** One heading of the file list, with the files filed under it. */
export interface Group {
  title: string;
  files: DawFileDto[];
}

/**
 * The bundle's files under four headings, in the order they are read: what
 * the piece *is*, then what it *sounds* like, then the record of both.
 *
 * A file the project wrote that fits no heading is still shown — under the
 * record — because a list that quietly dropped one would be the interface
 * disagreeing with the manifest about what is on disk.
 */
export function groups(report: DawReportDto): Group[] {
  const seen = new Set<string>();
  const take = (title: string, matches: (path: string) => boolean): Group => {
    const files = report.files.filter((file) => !seen.has(file.path) && matches(file.path));
    for (const file of files) seen.add(file.path);
    return { title, files };
  };
  const notation = take("Notation and MIDI", (path) => path.endsWith(".mid") || path.endsWith(".musicxml"));
  const mix = take("Mix", (path) => path === "audio/mix.wav");
  const stems = take("Stems", (path) => path.startsWith("audio/"));
  const record = take("Record", () => true);
  return [notation, mix, stems, record].filter((group) => group.files.length > 0);
}

/** The losses of one kind, gathered — a kind is a class, not an incident. */
export function byKind(report: DawReportDto): { kind: string; messages: string[] }[] {
  const kinds: { kind: string; messages: string[] }[] = [];
  for (const loss of report.losses) {
    const existing = kinds.find((each) => each.kind === loss.kind);
    if (existing) existing.messages.push(loss.message);
    else kinds.push({ kind: loss.kind, messages: [loss.message] });
  }
  return kinds;
}

/** A size a person reads, from the byte count the project measured. */
export function size(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} kB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** One line naming what was written and where. */
export function summary(report: DawReportDto): string {
  const count = report.files.length;
  const name = report.destination.split(/[/\\]/).pop() ?? report.destination;
  const losses = report.losses.length;
  const lost = losses === 0 ? "" : `, with ${losses} thing${losses === 1 ? "" : "s"} the formats cannot carry`;
  return `${count} file${count === 1 ? "" : "s"} in ${name}${lost}.`;
}

/** Whether a report says anything the composer has to weigh before importing. */
export function hasLosses(report: DawReportDto | null): report is DawReportDto {
  return report !== null && report.losses.length > 0;
}

export type { DawFileDto, DawLossDto, DawReportDto };
