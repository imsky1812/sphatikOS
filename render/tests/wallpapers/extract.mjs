// Exports the prototype's four wallpapers as standalone SVG files, using the
// prototype's own generator code (WALLS, RIBBONS, ribbonMarkup,
// crystalMarkup, wallMarkup in prototype/index.html), so the references are
// exactly what the prototype draws on its main screen (time tint off).
//
// Run from the repository root:  node render/tests/wallpapers/extract.mjs

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(here, "../../../prototype/index.html"), "utf8");

const start = html.indexOf("const WALLS = {");
const end = html.indexOf("function setWall(");
if (start < 0 || end < 0 || end <= start) {
  throw new Error("wallpaper code not found in prototype/index.html");
}
const code = html.slice(start, end);
const { WALLS, wallMarkup } = new Function(`${code}\nreturn { WALLS, wallMarkup };`)();

for (const key of Object.keys(WALLS)) {
  // Drop the procedural grain overlay (feTurbulence): its noise is
  // implementation-defined and cannot match a GPU render bit-for-bit, so the
  // Rust engine renders grain-free and the golden references do too.
  const body = wallMarkup(key, "", true).replace(
    /<rect[^>]*filter="url\(#grain\)"[^>]*\/>/g,
    "",
  );
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 393 852" width="393" height="852">` +
    body +
    `</svg>\n`;
  const out = join(here, `${key}.svg`);
  writeFileSync(out, svg);
  console.log(`wrote ${out} (${WALLS[key].name})`);
}
