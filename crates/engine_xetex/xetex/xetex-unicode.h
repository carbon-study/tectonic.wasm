/* tectonic/xetex-unicode.h: compact Unicode services for the WASM engine.
   Copyright 2026 the Tectonic Project
   Licensed under the MIT License. */

#ifndef TECTONIC_XETEX_UNICODE_H
#define TECTONIC_XETEX_UNICODE_H

#include <stdint.h>

typedef struct tt_grapheme_bidi tt_grapheme_bidi;

enum {
    TT_BIDI_LTR = 0,
    TT_BIDI_RTL = 1,
    TT_BIDI_MIXED = 2,
};

void tt_grapheme_linebreak_start(const uint16_t *text, int32_t text_length);
int32_t tt_grapheme_linebreak_next(void);

tt_grapheme_bidi *tt_grapheme_bidi_open(
    const uint16_t *text,
    int32_t text_length,
    int32_t default_direction
);
void tt_grapheme_bidi_close(tt_grapheme_bidi *bidi);
int32_t tt_grapheme_bidi_direction(const tt_grapheme_bidi *bidi);
int32_t tt_grapheme_bidi_count_runs(const tt_grapheme_bidi *bidi);
int32_t tt_grapheme_bidi_get_visual_run(
    const tt_grapheme_bidi *bidi,
    int32_t run_index,
    int32_t *logical_start,
    int32_t *length
);

int32_t tt_xetex_browser_decoder_available(const char *encoding);
int32_t tt_xetex_browser_decode(
    const char *encoding,
    const char *input,
    int32_t input_length,
    uint32_t *output,
    int32_t output_capacity
);

#endif
