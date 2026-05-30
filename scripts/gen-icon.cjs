// Generates a cute pixel Poro holding a magnifying glass (app's "reveal"
// theme). No image libs: hand-rolled PNG encoder. Background is a rounded-rect
// gradient; the Poro is a 32x16 left-half sprite mirrored for symmetry, with a
// code-drawn magnifier overlaid (so it can be asymmetric/diagonal).
// Writes scripts/app-icon.png (1024) and static/favicon.png (128).
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

// Left half (columns 0..15, col15 = centre). Mirrored to the right.
// . = background  F/f = fluff white/shadow  o = horn  K = eye  W = eye catch
// P = blush  M = tongue
const LEFT = [
  "................",
  "................",
  "................",
  "..........oo....",
  "..........oo....",
  "......FFFFFFFFFF",
  ".....FFFFFFFFFFF",
  "....FFFFFFFFFFFF",
  "...FFFFFFFFFFFFF",
  "...FFFFFFFFFFFFF",
  "..FFFFFFFFFFFFFF",
  "..FFFFFFFFFFFFFF",
  "..FFFFKKKFFFFFFF",
  "..FFFFKWKFFFFFFF",
  "..FFFFKKKFFFFFFF",
  "..FFFFFFFFFFFFFF",
  "..FFFFFFFFFFFFFF",
  "..FFFFFFFFFFFFMM",
  "..FFPPFFFFFFFFFF",
  "..FFFFFFFFFFFFFF",
  "...FFFFFFFFFFFFF",
  "...FFFFFFFFFFFFF",
  "....FFFFFFFFFFFF",
  "....FFFFFFFFFFFF",
  ".....FFFFFFFFFFF",
  "......FFFFFFFFFF",
  ".......FFFFFFFFF",
  "........FFFFFFFF",
  ".........FFFFFFF",
  "..........FFFFFF",
  "................",
  "................",
];

const PAL = {
  F: [255, 255, 255, 255],
  f: [222, 232, 244, 255],
  o: [214, 170, 112, 255],
  K: [60, 72, 110, 255],
  W: [255, 255, 255, 255],
  P: [255, 178, 203, 255],
  M: [255, 143, 163, 255],
};
const BG_TOP = [158, 200, 245];
const BG_BOT = [111, 168, 232];
const GRID = LEFT.map((row) => row + row.split("").reverse().join(""));
const N = 32;
const R = 5; // corner radius (grid units)

function inRounded(gx, gy) {
  const corners = [
    [R, R, gx < R && gy < R],
    [N - 1 - R, R, gx > N - 1 - R && gy < R],
    [R, N - 1 - R, gx < R && gy > N - 1 - R],
    [N - 1 - R, N - 1 - R, gx > N - 1 - R && gy > N - 1 - R],
  ];
  for (const [cx, cy, active] of corners) {
    if (active) {
      const dx = gx + 0.5 - cx;
      const dy = gy + 0.5 - cy;
      if (dx * dx + dy * dy > (R + 0.5) * (R + 0.5)) return false;
    }
  }
  return true;
}

function bgColor(gy) {
  const t = gy / (N - 1);
  return [
    Math.round(BG_TOP[0] + (BG_BOT[0] - BG_TOP[0]) * t),
    Math.round(BG_TOP[1] + (BG_BOT[1] - BG_TOP[1]) * t),
    Math.round(BG_TOP[2] + (BG_BOT[2] - BG_TOP[2]) * t),
    255,
  ];
}

// Magnifying glass overlaid on the lower-right, drawn in code so it can be
// diagonal/asymmetric (the mirrored sprite can't express that).
const MAG = { cx: 22, cy: 21, r: 5.2 };
const RING = [95, 208, 255, 255];
const HANDLE = [43, 127, 208, 255];
const GLASS = [191, 233, 255];

function cellColor(gx, gy) {
  const ch = GRID[gy][gx];
  let col;
  if (ch !== ".") col = PAL[ch].slice();
  else col = inRounded(gx, gy) ? bgColor(gy) : [0, 0, 0, 0];

  const dx = gx + 0.5 - MAG.cx;
  const dy = gy + 0.5 - MAG.cy;
  const dist = Math.sqrt(dx * dx + dy * dy);

  // Handle: thick diagonal extending down-right from the rim.
  if (
    dist > MAG.r &&
    dist <= MAG.r + 4.5 &&
    dx > 0 &&
    dy > 0 &&
    Math.abs(dx - dy) <= 1.2 &&
    inRounded(gx, gy)
  ) {
    col = HANDLE.slice();
  }

  // Lens: cyan rim + tinted glass (lets the Poro show through) + a shine.
  if (dist <= MAG.r) {
    if (dist >= MAG.r - 1.5) {
      col = RING.slice();
    } else if (col[3] === 0) {
      col = [GLASS[0], GLASS[1], GLASS[2], 255];
    } else {
      col = [
        Math.round(col[0] * 0.55 + GLASS[0] * 0.45),
        Math.round(col[1] * 0.55 + GLASS[1] * 0.45),
        Math.round(col[2] * 0.55 + GLASS[2] * 0.45),
        255,
      ];
    }
    if (dist < MAG.r - 1.5 && Math.abs(dx + 1.8) < 1 && Math.abs(dy + 1.8) < 1) {
      col = [255, 255, 255, 255];
    }
  }
  return col;
}

function crc32(buf) {
  if (!crc32.t) {
    const t = [];
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    crc32.t = t;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++)
    crc = crc32.t[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])), 0);
  return Buffer.concat([len, t, data, crc]);
}

function makePng(size) {
  const scale = size / N;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc(size * (1 + size * 4));
  let p = 0;
  for (let y = 0; y < size; y++) {
    raw[p++] = 0;
    const gy = Math.floor(y / scale);
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = cellColor(Math.floor(x / scale), gy);
      raw[p++] = r;
      raw[p++] = g;
      raw[p++] = b;
      raw[p++] = a;
    }
  }
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const root = path.join(__dirname, "..");
fs.writeFileSync(path.join(__dirname, "app-icon.png"), makePng(1024));
fs.writeFileSync(path.join(root, "static", "favicon.png"), makePng(128));
console.log("wrote scripts/app-icon.png (1024) and static/favicon.png (128)");
