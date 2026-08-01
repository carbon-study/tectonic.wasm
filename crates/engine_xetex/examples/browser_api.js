addToLibrary({
  tectonic_asyncify_checkpoint__async: 'auto',
  tectonic_asyncify_checkpoint: async () => {
    if (typeof globalThis.__tectonicCheckpoint !== 'function') {
      throw new Error('Tectonic checkpoint handler is not installed');
    }
    return await globalThis.__tectonicCheckpoint();
  },
  tectonic_asyncify_load__deps: ['$UTF8ToString', 'malloc'],
  tectonic_asyncify_load__async: 'auto',
  tectonic_asyncify_load: async (namePointer, isFormat, dataOut, lengthOut) => {
    const name = UTF8ToString(namePointer);
    const loader = globalThis.__tectonicLoadResource;
    if (typeof loader !== 'function') {
      console.error('[tectonic] globalThis.__tectonicLoadResource is not installed');
      return -1;
    }

    try {
      let bytes = await loader(name, Boolean(isFormat));
      if (bytes == null) {
        return 0;
      }
      if (!(bytes instanceof Uint8Array)) {
        bytes = new Uint8Array(bytes);
      }

      const buffer = _malloc(bytes.length);
      if (!buffer && bytes.length) {
        throw new Error(`malloc failed for ${bytes.length} bytes`);
      }
      HEAPU8.set(bytes, buffer);
      HEAPU32[dataOut >> 2] = buffer;
      HEAPU32[lengthOut >> 2] = bytes.length;
      return 1;
    } catch (error) {
      console.error(`[tectonic] resource load failed for ${name}:`, error);
      return -1;
    }
  },
});
