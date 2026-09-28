// Genere assets/icon.png (1024x1024 RGBA) sans dependance externe.
// Motif : carre arrondi degrade indigo -> cyan, avec trois barres (une "liste clients").
const zlib = require('zlib');
const fs = require('fs');

const S = 1024;
const buf = Buffer.alloc(S * S * 4, 0);

const clamp01 = (v) => Math.max(0, Math.min(1, v));
// Anti-aliasing : couverture d'un rectangle arrondi pour un point donne.
function roundRectCoverage(x, y, x0, y0, x1, y1, r) {
  const dx = Math.max(x0 + r - x, 0, x - (x1 - r));
  const dy = Math.max(y0 + r - y, 0, y - (y1 - r));
  const inRx = x >= x0 && x <= x1;
  const inRy = y >= y0 && y <= y1;
  if (!inRx || !inRy) {
    // distance signee approximative hors boite
    const ox = Math.max(x0 - x, 0, x - x1);
    const oy = Math.max(y0 - y, 0, y - y1);
    return clamp01(0.5 - Math.hypot(ox, oy));
  }
  const d = Math.hypot(dx, dy) - r;
  return clamp01(0.5 - d);
}

function put(i, r, g, b, a) {
  const sa = a + buf[i + 3] * (1 - a);
  if (sa <= 0) return;
  buf[i] = Math.round((r * a + buf[i] * (buf[i + 3] / 255) * (1 - a)) / sa * 255) & 255;
  buf[i + 1] = Math.round((g * a + buf[i + 1] * (buf[i + 3] / 255) * (1 - a)) / sa * 255) & 255;
  buf[i + 2] = Math.round((b * a + buf[i + 2] * (buf[i + 3] / 255) * (1 - a)) / sa * 255) & 255;
  buf[i + 3] = Math.round(sa * 255);
}

const M = 48;                 // marge
const R = 200;                // rayon des coins
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const i = (y * S + x) * 4;
    const cov = roundRectCoverage(x + 0.5, y + 0.5, M, M, S - M, S - M, R);
    if (cov <= 0) continue;
    const t = clamp01(((x - M) / (S - 2 * M)) * 0.45 + ((y - M) / (S - 2 * M)) * 0.55);
    // degrade #3B2E8F -> #1FB6D8
    const r = (0x3b + (0x1f - 0x3b) * t) / 255;
    const g = (0x2e + (0xb6 - 0x2e) * t) / 255;
    const b = (0x8f + (0xd8 - 0x8f) * t) / 255;
    put(i, r, g, b, cov);
  }
}

// Trois barres blanches + trois pastilles (la "table").
const bars = [
  { y: 340, dotW: 96, barW: 380 },
  { y: 480, dotW: 96, barW: 300 },
  { y: 620, dotW: 96, barW: 348 },
];
const H = 74, BR = 37, X0 = 236, GAP = 42;
for (const bar of bars) {
  for (let y = bar.y - 4; y < bar.y + H + 4; y++) {
    for (let x = X0 - 4; x < X0 + bar.dotW + GAP + bar.barW + 4; x++) {
      if (x < 0 || y < 0 || x >= S || y >= S) continue;
      const i = (y * S + x) * 4;
      const c1 = roundRectCoverage(x + 0.5, y + 0.5, X0, bar.y, X0 + bar.dotW, bar.y + H, BR);
      const c2 = roundRectCoverage(x + 0.5, y + 0.5, X0 + bar.dotW + GAP, bar.y, X0 + bar.dotW + GAP + bar.barW, bar.y + H, BR);
      // Pastille et barre au meme blanc plein : a 32 px, une barre attenuee se
      // noie dans le degrade et l'icone devient illisible.
      const c = Math.max(c1, c2);
      if (c > 0) put(i, 1, 1, 1, c);
    }
  }
}

// Encodage PNG
const raw = Buffer.alloc((S * 4 + 1) * S);
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // filtre None
  buf.copy(raw, y * (S * 4 + 1) + 1, y * S * 4, (y + 1) * S * 4);
}
const crcTable = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
})();
function crc32(b) {
  let c = -1;
  for (let i = 0; i < b.length; i++) c = crcTable[(c ^ b[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0); ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);
fs.writeFileSync(__dirname + '/icon.png', png);
console.log('icon.png ecrit :', png.length, 'octets');
