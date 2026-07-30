/* tectonic/xetex-unicode.c: compact Unicode services for the WASM engine.
   Copyright 2026 the Tectonic Project
   Licensed under the MIT License. */

#include "xetex-unicode.h"

#include <grapheme.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdlib.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

typedef struct {
    int32_t logical_start;
    int32_t length;
    int32_t level;
} bidi_run;

struct tt_grapheme_bidi {
    bidi_run *runs;
    int32_t run_count;
    int32_t direction;
};

typedef struct {
    uint32_t *codepoints;
    int32_t *starts;
    int32_t *ends;
    size_t length;
} utf32_text;

static utf32_text linebreak_text;
static size_t linebreak_offset;

extern int tectonic_grapheme_bidi_resolve_direction(
    const uint_least32_t *text,
    size_t length,
    int fallback
);

static void
utf32_text_clear(utf32_text *text)
{
    free(text->codepoints);
    free(text->starts);
    free(text->ends);
    text->codepoints = NULL;
    text->starts = NULL;
    text->ends = NULL;
    text->length = 0;
}

static bool
utf32_text_from_utf16(const uint16_t *input, int32_t input_length, utf32_text *output)
{
    int32_t i;
    size_t out;

    output->codepoints = calloc((size_t) input_length, sizeof(uint32_t));
    output->starts = calloc((size_t) input_length, sizeof(int32_t));
    output->ends = calloc((size_t) input_length, sizeof(int32_t));
    output->length = 0;

    if (input_length > 0 &&
        (output->codepoints == NULL || output->starts == NULL || output->ends == NULL)) {
        utf32_text_clear(output);
        return false;
    }

    for (i = 0, out = 0; i < input_length; i++, out++) {
        uint32_t cp = input[i];
        output->starts[out] = i;

        if (cp >= UINT32_C(0xD800) && cp <= UINT32_C(0xDBFF) &&
            i + 1 < input_length &&
            input[i + 1] >= UINT32_C(0xDC00) &&
            input[i + 1] <= UINT32_C(0xDFFF)) {
            cp = UINT32_C(0x10000) +
                 ((cp - UINT32_C(0xD800)) << 10) +
                 (input[++i] - UINT32_C(0xDC00));
        } else if (cp >= UINT32_C(0xD800) && cp <= UINT32_C(0xDFFF)) {
            cp = GRAPHEME_INVALID_CODEPOINT;
        }

        output->codepoints[out] = cp;
        output->ends[out] = i + 1;
    }

    output->length = out;
    return true;
}

void
tt_grapheme_linebreak_start(const uint16_t *text, int32_t text_length)
{
    utf32_text_clear(&linebreak_text);
    linebreak_offset = 0;
    utf32_text_from_utf16(text, text_length, &linebreak_text);
}

int32_t
tt_grapheme_linebreak_next(void)
{
    size_t count;

    if (linebreak_offset >= linebreak_text.length) {
        return -1;
    }

    count = grapheme_next_line_break(
        linebreak_text.codepoints + linebreak_offset,
        linebreak_text.length - linebreak_offset
    );
    if (count == 0 || count > linebreak_text.length - linebreak_offset) {
        return -1;
    }

    linebreak_offset += count;
    return linebreak_text.ends[linebreak_offset - 1];
}

static void
reverse_runs(bidi_run *runs, int32_t first, int32_t last)
{
    while (first < last) {
        bidi_run tmp = runs[first];
        runs[first++] = runs[last];
        runs[last--] = tmp;
    }
}

tt_grapheme_bidi *
tt_grapheme_bidi_open(const uint16_t *text, int32_t text_length, int32_t default_direction)
{
    utf32_text converted = { 0 };
    enum grapheme_bidirectional_direction requested;
    enum grapheme_bidirectional_direction resolved;
    tt_grapheme_bidi *bidi;
    uint32_t *state;
    int8_t *levels;
    size_t processed;
    size_t i;
    int32_t paragraph_level;
    int32_t max_level = 0;
    int32_t min_odd_level = 126;
    int32_t run_count = 0;
    int32_t level;

    if (!utf32_text_from_utf16(text, text_length, &converted)) {
        return NULL;
    }

    bidi = calloc(1, sizeof(*bidi));
    state = calloc(converted.length, sizeof(*state));
    levels = calloc(converted.length + 1, sizeof(*levels));
    if (bidi == NULL ||
        (converted.length > 0 && (state == NULL || levels == NULL))) {
        free(bidi);
        free(state);
        free(levels);
        utf32_text_clear(&converted);
        return NULL;
    }

    requested = (enum grapheme_bidirectional_direction)
        tectonic_grapheme_bidi_resolve_direction(
            converted.codepoints,
            converted.length,
            default_direction == 0xFF ?
                GRAPHEME_BIDIRECTIONAL_DIRECTION_RTL :
                GRAPHEME_BIDIRECTIONAL_DIRECTION_LTR
        );
    processed = grapheme_bidirectional_preprocess_paragraph(
        converted.codepoints,
        converted.length,
        requested,
        state,
        converted.length,
        &resolved
    );
    if (processed < converted.length) {
        converted.length = processed;
    }

    grapheme_bidirectional_get_line_embedding_levels(
        state,
        converted.length,
        levels,
        converted.length
    );

    paragraph_level =
        resolved == GRAPHEME_BIDIRECTIONAL_DIRECTION_RTL ? 1 : 0;

    /* Formatting controls removed by X9 still occupy UTF-16 input positions.
     * Attach them to a neighboring run so that HarfBuzz sees the same slice
     * boundaries as the original text. */
    level = paragraph_level;
    for (i = converted.length; i > 0; i--) {
        if (levels[i - 1] < 0) {
            levels[i - 1] = level;
        } else {
            level = levels[i - 1];
        }
    }

    for (i = 0; i < converted.length; i++) {
        if (i == 0 || levels[i] != levels[i - 1]) {
            run_count++;
        }
        if (levels[i] > max_level) {
            max_level = levels[i];
        }
        if ((levels[i] & 1) != 0 && levels[i] < min_odd_level) {
            min_odd_level = levels[i];
        }
    }

    bidi->runs = calloc((size_t) run_count, sizeof(*bidi->runs));
    bidi->run_count = run_count;
    if (run_count > 0 && bidi->runs == NULL) {
        tt_grapheme_bidi_close(bidi);
        free(state);
        free(levels);
        utf32_text_clear(&converted);
        return NULL;
    }

    for (i = 0, run_count = 0; i < converted.length; i++) {
        if (i == 0 || levels[i] != levels[i - 1]) {
            bidi->runs[run_count].logical_start = converted.starts[i];
            bidi->runs[run_count].level = levels[i];
            if (run_count > 0) {
                bidi->runs[run_count - 1].length =
                    converted.starts[i] - bidi->runs[run_count - 1].logical_start;
            }
            run_count++;
        }
    }
    if (run_count > 0) {
        bidi->runs[run_count - 1].length =
            text_length - bidi->runs[run_count - 1].logical_start;
    }

    if (min_odd_level <= max_level) {
        for (level = max_level; level >= min_odd_level; level--) {
            int32_t first = 0;

            while (first < bidi->run_count) {
                int32_t last;
                while (first < bidi->run_count && bidi->runs[first].level < level) {
                    first++;
                }
                last = first;
                while (last < bidi->run_count && bidi->runs[last].level >= level) {
                    last++;
                }
                reverse_runs(bidi->runs, first, last - 1);
                first = last;
            }
        }
    }

    bidi->direction = paragraph_level == 0 ? TT_BIDI_LTR : TT_BIDI_RTL;
    for (i = 0; i < (size_t) bidi->run_count; i++) {
        if ((bidi->runs[i].level & 1) != paragraph_level) {
            bidi->direction = TT_BIDI_MIXED;
            break;
        }
    }

    free(state);
    free(levels);
    utf32_text_clear(&converted);
    return bidi;
}

void
tt_grapheme_bidi_close(tt_grapheme_bidi *bidi)
{
    if (bidi != NULL) {
        free(bidi->runs);
        free(bidi);
    }
}

int32_t
tt_grapheme_bidi_direction(const tt_grapheme_bidi *bidi)
{
    return bidi->direction;
}

int32_t
tt_grapheme_bidi_count_runs(const tt_grapheme_bidi *bidi)
{
    return bidi->run_count;
}

int32_t
tt_grapheme_bidi_get_visual_run(
    const tt_grapheme_bidi *bidi,
    int32_t run_index,
    int32_t *logical_start,
    int32_t *length
)
{
    const bidi_run *run = bidi->runs + run_index;
    *logical_start = run->logical_start;
    *length = run->length;
    return (run->level & 1) != 0 ? TT_BIDI_RTL : TT_BIDI_LTR;
}

#ifdef __EMSCRIPTEN__

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wmissing-variable-declarations"
#pragma clang diagnostic ignored "-Wextra-semi"

EM_JS(int32_t, tt_xetex_browser_decoder_available, (const char *encoding), {
    try {
        new TextDecoder(UTF8ToString(encoding), { fatal: true });
        return 1;
    } catch (_) {
        return 0;
    }
});

EM_JS(int32_t, tt_xetex_browser_decode, (
    const char *encoding,
    const char *input,
    int32_t input_length,
    uint32_t *output,
    int32_t output_capacity
), {
    try {
        const decoder = new TextDecoder(UTF8ToString(encoding), { fatal: true });
        const bytes = HEAPU8.subarray(input, input + input_length);
        const decoded = decoder.decode(bytes);
        let written = 0;

        for (const character of decoded) {
            if (written >= output_capacity) {
                return -2;
            }
            HEAPU32[(output >>> 2) + written] = character.codePointAt(0);
            written++;
        }
        return written;
    } catch (_) {
        return -1;
    }
});

#pragma clang diagnostic pop

#else

int32_t
tt_xetex_browser_decoder_available(const char *encoding)
{
    (void) encoding;
    return 0;
}

int32_t
tt_xetex_browser_decode(
    const char *encoding,
    const char *input,
    int32_t input_length,
    uint32_t *output,
    int32_t output_capacity
)
{
    (void) encoding;
    (void) input;
    (void) input_length;
    (void) output;
    (void) output_capacity;
    return -1;
}

#endif
