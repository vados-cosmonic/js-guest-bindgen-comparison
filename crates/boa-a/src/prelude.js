const WebAssemblyRuntimeError = Error;
// Boa cannot view the component's linear memory as an ArrayBuffer, so these
// replace jco's memory and string intrinsics with native functions (see lib.rs).
const $memory = { buffer: { byteLength: 4294967295 } };

function $realloc(oldPtr, oldSize, align, newSize) {
  if (oldPtr !== 0) throw new Error('realloc of an existing allocation is unsupported');
  return __wit_alloc(newSize);
}

function $dealloc(ptr) {
  __wit_free(ptr);
}

const $wit_view = {
  getInt8: p => __wit_load(0, p),
  getUint8: p => __wit_load(1, p),
  getInt16: p => __wit_load(2, p),
  getUint16: p => __wit_load(3, p),
  getInt32: p => __wit_load(4, p),
  getUint32: p => __wit_load(5, p),
  getFloat32: p => __wit_load(6, p),
  getFloat64: p => __wit_load(7, p),
  setInt8: (p, v) => __wit_store(0, p, v),
  setUint8: (p, v) => __wit_store(1, p, v),
  setInt16: (p, v) => __wit_store(2, p, v),
  setUint16: (p, v) => __wit_store(3, p, v),
  setInt32: (p, v) => __wit_store(4, p, v),
  setUint32: (p, v) => __wit_store(5, p, v),
  setFloat32: (p, v) => __wit_store(6, p, v),
  setFloat64: (p, v) => __wit_store(7, p, v),
  getBigInt64() { throw new Error('64-bit integers are unsupported'); },
  setBigInt64() { throw new Error('64-bit integers are unsupported'); }
};
const dataView = mem => $wit_view;

// copies instead of viewing
function memoryView(ty, mem, ptr, len) {
  return new ty(__wit_read(ptr, len * ty.BYTES_PER_ELEMENT), 0, len);
}

function memoryWrite(mem, ptr, bytes) {
  __wit_write(ptr, bytes);
}

const TEXT_DECODER_UTF8 = { decode: bytes => __wit_utf8_decode(bytes) };

function _utf8AllocateAndEncode(s, realloc, memory) {
  if (typeof s !== 'string') throw new TypeError('expected a string, received [' + typeof s + ']');
  const [ptr, len] = __wit_utf8_encode(s);
  return { ptr, len };
}
