// Generates a clean flat/vector app icon: a white magnifying glass on a
// blue->cyan gradient squircle. Rendered with 4x supersampled anti-aliasing
// straight to PNG (hand-rolled encoder, no image libs) so edges are smooth.
// Writes scripts/app-icon.png (1024) and static/favicon.png (128).
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const S = 1024; // scene units

// palette
const GRAD_A = [74, 124, 247]; // blue (top-left)
const GRAD_B = [56, 201, 240]; // cyan (bottom-right)
const WHITE = [255, 255, 255];

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

// signed distance to a rounded rectangle centred in the canvas
function roundRect(x, y, half, r) {
  const qx = Math.abs(x - S / 2) - (half - r);
  const qy = Math.abs(y - S / 2) - (half - r);
  const ax = Math.max(qx, 0),
    ay = Math.max(qy, 0);
  return Math.hypot(ax, ay) + Math.min(Math.max(qx, qy), 0) - r;
}

function segDist(px, py, ax, ay, bx, by) {
  const dx = bx - ax,
    dy = by - ay,
    l2 = dx * dx + dy * dy;
  let t = l2 ? ((px - ax) * dx + (py - ay) * dy) / l2 : 0;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

// magnifier geometry
const LCX = 430,
  LCY = 412,
  R_OUT = 236,
  R_IN = 150;
const HAX = LCX + R_OUT * 0.707,
  HAY = LCY + R_OUT * 0.707;
const HBX = 824,
  HBY = 806,
  HW = 60;

function scene(x, y) {
  let col = [0, 0, 0, 0];

  // squircle background with diagonal gradient
  if (roundRect(x, y, S / 2, 232) < 0) {
    const t = (x + y) / (2 * S);
    col = mix(GRAD_A, GRAD_B, t).concat(255);
  }

  const d = Math.hypot(x - LCX, y - LCY);

  // glass: faint white fill so the lens reads as glass over the gradient
  if (d < R_IN) col = over(col, [255, 255, 255, 50]);

  // ring (white)
  if (d >= R_IN && d <= R_OUT) col = [255, 255, 255, 255];

  // handle (white capsule)
  if (segDist(x, y, HAX, HAY, HBX, HBY) < HW) col = [255, 255, 255, 255];

  // shine streaks inside the glass (upper-left)
  if (d < R_IN - 8) {
    if (segDist(x, y, 352, 352, 408, 408) < 22)
      col = over(col, [255, 255, 255, 210]);
    if (segDist(x, y, 350, 420, 372, 442) < 11)
      col = over(col, [255, 255, 255, 210]);
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
  const ss = 4; // supersamples per axis
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
