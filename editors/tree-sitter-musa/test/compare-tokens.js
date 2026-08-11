#!/usr/bin/env node
/**
 * The corpus side of the drift law.
 *
 * `crates/musa-language/tests/tree_sitter_fixtures.rs` commits the *real*
 * lexer's token stream for every compilable fixture, and the *real*
 * parser's syntax verdict on every broken one. This script holds the
 * tree-sitter grammar to both: it parses each fixture through the CLI's
 * `--cst`, reads the leaves off with their source text, and compares them
 * against the committed stream token for token. A grammar that disagrees
 * with the lexer about a single token fails here — in CI, not in an editor.
 *
 * Run with `node test/compare-tokens.js` (or `npm test`).
 */

'use strict';

const { spawnSync } = require('node:child_process');
const { readFileSync, readdirSync } = require('node:fs');
const { join, basename } = require('node:path');

const GRAMMAR = join(__dirname, '..');
const REPO = join(GRAMMAR, '..', '..');
const EXAMPLES = join(REPO, 'examples');
const TOKENS = join(GRAMMAR, 'test', 'tokens');

let failures = 0;

function fail(message) {
  failures += 1;
  console.error(`not ok — ${message}`);
}

/** Parse `path` with the CLI: the CST text, the exit code, and the crash check. */
function parse(path) {
  const result = spawnSync('tree-sitter', ['parse', '--cst', '--quiet', path], {
    cwd: GRAMMAR,
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1' },
  });
  if (result.error) {
    fail(`${basename(path)}: tree-sitter could not run: ${result.error.message}`);
    return null;
  }
  if (result.status === null) {
    fail(`${basename(path)}: the parser crashed (signal ${result.signal}) — editors run it on every keystroke`);
    return null;
  }
  return { cst: result.stdout, hadErrors: result.status !== 0 };
}

/**
 * The leaf tokens of a CST dump, with their text sliced back out of the
 * source by byte offset (the dump escapes the text it shows, and tree-sitter
 * points count bytes, not UTF-16 cells). Lines look like:
 *
 *   0:0   - 20:0    source_file
 *   0:0   - 0:5       "piece"
 *   0:6   - 0:15      name: string `\"twinkle\"`
 *
 * where depth is the indent before the node, a field prefix may precede the
 * kind, and a node is a leaf when the next line does not indent past it.
 */
function leaves(cst, source) {
  const pattern = /^((\d+):(\d+)\s+-\s+(\d+):(\d+))(\s+)(\S.*)$/;
  const rows = [];
  for (const line of cst.split('\n')) {
    const match = pattern.exec(line);
    if (!match) continue;
    const [, positions, row1, col1, row2, col2, indent, rest] = match;
    let kind;
    if (rest.startsWith('"')) {
      // An anonymous token, named by its text between the quotes — and none
      // of the language's keywords or punctuation contains a quote.
      kind = rest.slice(1, rest.lastIndexOf('"'));
    } else {
      kind = rest.replace(/^(\w+):\s+/, '').split(/\s|`/)[0];
    }
    rows.push({
      start: [Number(row1), Number(col1)],
      end: [Number(row2), Number(col2)],
      // The dump pads the position column to the widest span on screen, so
      // depth is the column the node text starts at, not the raw indent.
      depth: positions.length + indent.length,
      kind,
    });
  }
  const lines = source.split('\n').map((line) => Buffer.from(line, 'utf8'));
  const textAt = ({ start, end }) => {
    if (start[0] === end[0]) return lines[start[0]].subarray(start[1], end[1]).toString('utf8');
    const parts = [lines[start[0]].subarray(start[1]).toString('utf8')];
    for (let row = start[0] + 1; row < end[0]; row += 1) parts.push(lines[row].toString('utf8'));
    parts.push(lines[end[0]].subarray(0, end[1]).toString('utf8'));
    return parts.join('\n');
  };
  const out = [];
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index];
    const next = rows[index + 1];
    if (next && next.depth > row.depth) continue; // an internal node
    out.push({ kind: row.kind, text: textAt(row) });
  }
  return out;
}

/** Compare one fixture's leaves against its committed lexer stream. */
function checkTokens(manifestPath) {
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const name = basename(manifestPath, '.json');
  const sourcePath = join(EXAMPLES, manifest.file);
  const source = readFileSync(sourcePath, 'utf8');
  const parsed = parse(sourcePath);
  if (!parsed) return;
  if (parsed.hadErrors) {
    fail(`${name}: a compilable fixture parsed with errors — the grammar is behind the language`);
    return;
  }
  const actual = leaves(parsed.cst, source);
  const expected = manifest.tokens;
  if (actual.length !== expected.length) {
    fail(`${name}: ${actual.length} leaves against the lexer's ${expected.length} tokens`);
  }
  for (let index = 0; index < Math.min(actual.length, expected.length); index += 1) {
    const got = actual[index];
    const want = expected[index];
    if (got.kind !== want.kind || got.text !== want.text) {
      fail(
        `${name}: token ${index} is (${got.kind}, ${JSON.stringify(got.text)}) ` +
          `where the lexer wrote (${want.kind}, ${JSON.stringify(want.text)})`,
      );
      return; // one disagreement is enough to read; the rest is noise
    }
  }
}

/** Hold the grammar to the real parser's verdict on each broken fixture. */
function checkBroken() {
  const manifest = JSON.parse(readFileSync(join(GRAMMAR, 'test', 'broken.json'), 'utf8'));
  for (const fixture of manifest.fixtures) {
    const path = join(EXAMPLES, 'broken', `${fixture.name}.musa`);
    const parsed = parse(path);
    if (!parsed) continue;
    if (parsed.hadErrors !== fixture.syntax_errors) {
      fail(
        `${fixture.name}: the grammar ${parsed.hadErrors ? 'found errors' : 'parsed clean'} ` +
          `where the real parser ${fixture.syntax_errors ? 'found errors' : 'parsed clean'}`,
      );
    }
  }
}

for (const file of readdirSync(TOKENS).filter((file) => file.endsWith('.json')).sort()) {
  checkTokens(join(TOKENS, file));
}
checkBroken();

if (failures > 0) {
  console.error(`\n${failures} drift-law failure(s)`);
  process.exit(1);
}
console.log('drift law holds: every token and every verdict agrees with musa-language');
