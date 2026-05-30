// Generates a pixel-art magnifying-glass app icon (no image libs required).
// Renders a 16x16 grid scaled up, encodes a PNG by hand, and writes:
//   - scripts/app-icon.png   (1024x1024 source for `tauri icon`)
//   - static/favicon.png     (128x128 web favicon)
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

// 16x16 pixel map. Digits index into PALETTE below.
// Lens (top-left) + handle (bottom-right) = a magnifying glass.
const GRID = [
  "0011111111111100",
  "0111111111111110",
  "1111333311111111",
  "1113575531111111",
  "1135444453111111",
  "1134444443111111",
  "1134444443111111",
  "1133444443311111",
  "1113344433611111",
  "1111333336661111",
  "1111111166661111",
  "1111111116666111",
  "1111111111666111",
  "1111111111166111",
  "0111111111111110",
  "0011111111111100",
].map((r) => r.split("").map(Number));

const PALETTE = {
  0: [0, 0, 0, 0], // transparent
  1: [22, 27, 39, 255], // bg navy   #161b27
  3: [95, 208, 255, 255], // ring     #5fd0ff
  4: [42, 108, 240, 255], // glass    #2a6cf0
  5: [127, 180, 255, 255], // glass hl #7fb4ff
  6: [95, 208, 255, 255], // handle   #5fd0ff
  7: [255, 255, 255, 255], // shine
};

function crc32(buf) {
  if (!crc32.table) {
    const t = [];
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    crc32.table = t;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++)
    crc = crc32.table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function makePng(size) {
  const scale = size / 16;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(size * (1 + size * 4));
  let p = 0;
  for (let y = 0; y < size; y++) {
    raw[p++] = 0; // filter: none
    const gy = Math.floor(y / scale);
    for (let x = 0; x < size; x++) {
      const gx = Math.floor(x / scale);
      const [r, g, b, a] = PALETTE[GRID[gy][gx]];
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
