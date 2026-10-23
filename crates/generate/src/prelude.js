const WebAssemblyRuntimeError = Error;
// porffor has no wasm memory object: pointers are absolute linear memory
// addresses, accessed through Porffor.c
const $memory = { buffer: { byteLength: 4294967295 } };

function $realloc(oldPtr, oldSize, align, newSize) {
  if (oldPtr !== 0) throw new Error('wit: realloc of an existing allocation is unsupported');
  let r = 0;
  Porffor.c`WIT_SET(r, (f64)wit_js_realloc(WIT_U32(newSize)));`;
  return r;
}

function $dealloc(ptr) {
  Porffor.c`free((void*)(uintptr_t)WIT_U32(ptr));`;
}

const $wit_view = {
  getInt8(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(int8_t, WIT_U32(p)));`; return r; },
  getUint8(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(uint8_t, WIT_U32(p)));`; return r; },
  getInt16(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(int16_t, WIT_U32(p)));`; return r; },
  getUint16(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(uint16_t, WIT_U32(p)));`; return r; },
  getInt32(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(int32_t, WIT_U32(p)));`; return r; },
  getUint32(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(uint32_t, WIT_U32(p)));`; return r; },
  getFloat32(p) { let r = 0; Porffor.c`WIT_SET(r, (f64)WIT_LD(float, WIT_U32(p)));`; return r; },
  getFloat64(p) { let r = 0; Porffor.c`WIT_SET(r, WIT_LD(double, WIT_U32(p)));`; return r; },
  getBigInt64(p) { throw new Error('wit: 64-bit integers are unsupported'); },
  setInt8(p, v) { Porffor.c`WIT_ST(int8_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setUint8(p, v) { Porffor.c`WIT_ST(uint8_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setInt16(p, v) { Porffor.c`WIT_ST(int16_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setUint16(p, v) { Porffor.c`WIT_ST(uint16_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setInt32(p, v) { Porffor.c`WIT_ST(int32_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setUint32(p, v) { Porffor.c`WIT_ST(uint32_t, WIT_U32(p), (int64_t)WIT_NUM(v));`; },
  setFloat32(p, v) { Porffor.c`WIT_ST(float, WIT_U32(p), WIT_NUM(v));`; },
  setFloat64(p, v) { Porffor.c`WIT_ST(double, WIT_U32(p), WIT_NUM(v));`; },
  setBigInt64(p, v) { throw new Error('wit: 64-bit integers are unsupported'); }
};
const dataView = mem => $wit_view;

// copies instead of viewing, since a typed array cannot alias linear memory
function memoryView(ty, mem, ptr, len) {
  const out = new ty(len);
  const witMemoryBytes = out.byteLength;
  Porffor.c`memcpy(WIT_TA_DATA(out), (void*)(uintptr_t)WIT_U32(ptr), WIT_U32(witMemoryBytes));`;
  return out;
}

function memoryWrite(mem, ptr, witMemoryBytes) {
  const n = witMemoryBytes.byteLength;
  Porffor.c`memcpy((void*)(uintptr_t)WIT_U32(ptr), WIT_TA_DATA(witMemoryBytes), WIT_U32(n));`;
}

const TEXT_DECODER_UTF8 = {
  decode(b) {
    let s = '';
    for (let i = 0; i < b.length;) {
      let c = b[i++];
      if (c >= 0xf0) c = ((c & 0x07) << 18) | ((b[i++] & 0x3f) << 12) | ((b[i++] & 0x3f) << 6) | (b[i++] & 0x3f);
      else if (c >= 0xe0) c = ((c & 0x0f) << 12) | ((b[i++] & 0x3f) << 6) | (b[i++] & 0x3f);
      else if (c >= 0xc0) c = ((c & 0x1f) << 6) | (b[i++] & 0x3f);
      if (c >= 0x10000) {
        c -= 0x10000;
        s += String.fromCharCode(0xd800 + (c >> 10), 0xdc00 + (c & 0x3ff));
      } else s += String.fromCharCode(c);
    }
    return s;
  }
};

function _utf8AllocateAndEncode(s, realloc, memory) {
  if (typeof s !== 'string') throw new TypeError('expected a string, received [' + typeof s + ']');
  const cps = [];
  for (let i = 0; i < s.length; i++) {
    let c = s.charCodeAt(i);
    if (c >= 0xd800 && c <= 0xdbff && i + 1 < s.length) {
      const d = s.charCodeAt(i + 1);
      if (d >= 0xdc00 && d <= 0xdfff) {
        c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00);
        i++;
      }
    }
    if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd;
    cps.push(c);
  }
  let len = 0;
  for (const c of cps) len += c < 0x80 ? 1 : c < 0x800 ? 2 : c < 0x10000 ? 3 : 4;
  const witMemoryBytes = new Uint8Array(len);
  let j = 0;
  for (const c of cps) {
    if (c < 0x80) witMemoryBytes[j++] = c;
    else if (c < 0x800) {
      witMemoryBytes[j++] = 0xc0 | (c >> 6);
      witMemoryBytes[j++] = 0x80 | (c & 0x3f);
    } else if (c < 0x10000) {
      witMemoryBytes[j++] = 0xe0 | (c >> 12);
      witMemoryBytes[j++] = 0x80 | ((c >> 6) & 0x3f);
      witMemoryBytes[j++] = 0x80 | (c & 0x3f);
    } else {
      witMemoryBytes[j++] = 0xf0 | (c >> 18);
      witMemoryBytes[j++] = 0x80 | ((c >> 12) & 0x3f);
      witMemoryBytes[j++] = 0x80 | ((c >> 6) & 0x3f);
      witMemoryBytes[j++] = 0x80 | (c & 0x3f);
    }
  }
  const ptr = realloc(0, 0, 1, len);
  memoryWrite(memory, ptr, witMemoryBytes);
  return { ptr, len };
}
