// Generates a clean flat/vector app icon: a cute Poro on a blue->cyan gradient
// squircle. Rendered with 4x supersampled anti-aliasing straight to PNG
// (hand-rolled encoder, no image libs) so edges are smooth.
// Writes scripts/app-icon.png (1024) and static/favicon.png (128).
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const S = 1024; // scene units

// palette
const GRAD_A = [74, 124, 247]; // blue (top-left)
const GRAD_B = [56, 201, 240]; // cyan (bottom-right)
const WHITE = [255, 255, 255];
const BODY_SHADE = [228, 236, 248];
const OUTLINE = [120, 162, 214];
const HORN = [226, 182, 120];
const EYE = [40, 50, 78];
const BLUSH = [255, 150, 180];
const MOUTH = [112, 64, 80];
const TONGUE = [255, 138, 160];

const mix = (a, b, t) => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];
function over(dst, src) {
  const sa = src[3] / 255,
    da = dst[3] / 255;
  const oa = sa + da * (1 - sa);
  if (oa <= 0) return [0, 0, 0, 0];
  return [
    (src[0] * sa + dst[0] * da * (1 - sa)) / oa,
    (src[1] * sa + dst[1] * da * (1 - sa)) / oa,
    (src[2] * sa + dst[2] * da * (1 - sa)) / oa,
    oa * 255,
  ];
}
const dist = (x, y, a, b) => Math.hypot(x - a, y - b);
// normalized superellipse radius (<1 inside)
function se(x, y, cx, cy, rx, ry, n) {
  return (
    Math.pow(Math.abs((x - cx) / rx), n) + Math.pow(Math.abs((y - cy) / ry), n)
  );
}
function roundRect(x, y, half, r) {
  const qx = Math.abs(x - S / 2) - (half - r);
  const qy = Math.abs(y - S / 2) - (half - r);
  const ax = Math.max(qx, 0),
    ay = Math.max(qy, 0);
  return Math.hypot(ax, ay) + Math.min(Math.max(qx, qy), 0) - r;
}

// geometry
const BCX = 512,
  BCY = 552,
  BRX = 270,
  BRY = 298,
  BN = 2.3;
const EYES = [430, 594];
const EY = 540;

function scene(x, y) {
  let col = [0, 0, 0, 0];

  // squircle background, diagonal gradient
  if (roundRect(x, y, S / 2, 232) < 0) {
    col = mix(GRAD_A, GRAD_B, (x + y) / (2 * S)).concat(255);
  }

  // horn tufts (behind body; tips peek above)
  if (se(x, y, 448, 236, 40, 60, 2) < 1 || se(x, y, 576, 236, 40, 60, 2) < 1) {
    col = HORN.concat(255);
  }

  // body (egg) with subtle shading + soft outline
  const bv = se(x, y, BCX, BCY, BRX, BRY, BN);
  if (bv < 1) {
    const t = Math.max(0, Math.min(0.45, (y - (BCY - BRY)) / (2 * BRY)));
    col = mix(WHITE, BODY_SHADE, t).concat(255);

    // blush (soft radial)
    for (const bx of [368, 656]) {
      const bd = Math.hypot((x - bx) / 48, (y - 612) / 30);
      if (bd < 1) col = mix(col, BLUSH, 0.55 * (1 - bd)).concat(255);
    }

    // eyes + catchlights
    for (const ex of EYES) {
      if (se(x, y, ex, EY, 46, 58, 2) < 1) {
        col = EYE.concat(255);
        if (dist(x, y, ex - 17, EY - 22) < 22) col = WHITE.concat(255);
        if (dist(x, y, ex + 14, EY + 22) < 11) col = WHITE.concat(255);
      }
    }

    // mouth + tongue
    if (se(x, y, BCX, 614, 22, 15, 2) < 1) col = MOUTH.concat(255);
    if (se(x, y, BCX, 624, 15, 10, 2) < 1) col = TONGUE.concat(255);
  } else if (bv < 1.12) {
    col = over(col, OUTLINE.concat(235));
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
  const ss = 4;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc(size * (1 + size * 4));
  let p = 0;
  const scale = S / size;
  for (let y = 0; y < size; y++) {
    raw[p++] = 0;
    for (let x = 0; x < size; x++) {
      let r = 0,
        g = 0,
        b = 0,
        a = 0;
      for (let sy = 0; sy < ss; sy++) {
        for (let sx = 0; sx < ss; sx++) {
          const c = scene(
            (x + (sx + 0.5) / ss) * scale,
            (y + (sy + 0.5) / ss) * scale,
          );
          r += c[0] * c[3];
          g += c[1] * c[3];
          b += c[2] * c[3];
          a += c[3];
        }
      }
      const n = ss * ss;
      raw[p++] = a > 0 ? Math.round(r / a) : 0;
      raw[p++] = a > 0 ? Math.round(g / a) : 0;
      raw[p++] = a > 0 ? Math.round(b / a) : 0;
      raw[p++] = Math.round(a / n);
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
