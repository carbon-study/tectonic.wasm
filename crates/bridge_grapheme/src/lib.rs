// Copyright 2026 the Tectonic Project
// Licensed under the MIT License.

//! A narrow bridge to libgrapheme's Unicode line-breaking and bidi
//! implementations.

/// Base direction for the Unicode bidi algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum BidiDirection {
    /// Resolve the paragraph direction from its contents.
    Neutral = 0,
    /// Force left-to-right paragraph direction.
    LeftToRight = 1,
    /// Force right-to-left paragraph direction.
    RightToLeft = 2,
}

unsafe extern "C" {
    fn grapheme_next_character_break(text: *const u32, length: usize) -> usize;
    fn grapheme_next_line_break(text: *const u32, length: usize) -> usize;
    fn grapheme_bidirectional_preprocess_paragraph(
        source: *const u32,
        source_length: usize,
        direction: BidiDirection,
        state: *mut u32,
        state_length: usize,
        resolved_direction: *mut BidiDirection,
    ) -> usize;
    fn grapheme_bidirectional_get_line_embedding_levels(
        state: *const u32,
        state_length: usize,
        levels: *mut i8,
        levels_length: usize,
    ) -> usize;
    fn grapheme_bidirectional_reorder_line(
        text: *const u32,
        state: *const u32,
        length: usize,
        output: *mut u32,
        output_length: usize,
    ) -> usize;
}

/// Return the number of code points through the next UAX #29 grapheme cluster.
pub fn next_character_break(text: &[u32]) -> usize {
    // SAFETY: libgrapheme reads exactly `text.len()` code points and does not
    // retain the pointer.
    unsafe { grapheme_next_character_break(text.as_ptr(), text.len()) }
}

/// Return the number of code points through the next UAX #14 line boundary.
pub fn next_line_break(text: &[u32]) -> usize {
    // SAFETY: libgrapheme reads exactly `text.len()` code points and does not
    // retain the pointer.
    unsafe { grapheme_next_line_break(text.as_ptr(), text.len()) }
}

/// Preprocess one paragraph and return its opaque bidi state and direction.
pub fn preprocess_bidi_paragraph(
    text: &[u32],
    direction: BidiDirection,
) -> (Vec<u32>, BidiDirection) {
    let mut state = vec![0; text.len()];
    let mut resolved_direction = BidiDirection::Neutral;

    // SAFETY: All pointers refer to valid slices of the stated lengths.
    let required = unsafe {
        grapheme_bidirectional_preprocess_paragraph(
            text.as_ptr(),
            text.len(),
            direction,
            state.as_mut_ptr(),
            state.len(),
            &mut resolved_direction,
        )
    };
    state.truncate(required.min(state.len()));

    (state, resolved_direction)
}

/// Return resolved embedding levels for preprocessed bidi state.
pub fn bidi_embedding_levels(state: &[u32]) -> Vec<i8> {
    // libgrapheme currently checks one element past the advertised capacity
    // while writing, so reserve a sentinel until that upstream bound is fixed.
    let mut levels = vec![0; state.len().saturating_add(1)];

    // SAFETY: Both pointers refer to valid buffers, including the extra
    // sentinel described above.
    let required = unsafe {
        grapheme_bidirectional_get_line_embedding_levels(
            state.as_ptr(),
            state.len(),
            levels.as_mut_ptr(),
            state.len(),
        )
    };
    levels.truncate(required.min(state.len()));
    levels
}

/// Reorder and mirror a line using its preprocessed bidi state.
pub fn reorder_bidi_line(text: &[u32], state: &[u32]) -> Vec<u32> {
    assert_eq!(text.len(), state.len());
    let mut output = vec![0; text.len().saturating_add(1)];

    // SAFETY: All pointers refer to valid slices of the stated lengths.
    let required = unsafe {
        grapheme_bidirectional_reorder_line(
            text.as_ptr(),
            state.as_ptr(),
            text.len(),
            output.as_mut_ptr(),
            output.len(),
        )
    };
    output.truncate(required.min(output.len()));
    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_ascii_word_boundary() {
        let text: Vec<u32> = "hello world".chars().map(u32::from).collect();
        assert_eq!(super::next_line_break(&text), 6);
    }

    #[test]
    fn keeps_combining_sequence_in_one_grapheme() {
        let text: Vec<u32> = "a\u{301}b".chars().map(u32::from).collect();
        assert_eq!(super::next_character_break(&text), 2);
    }

    #[test]
    fn resolves_and_reorders_mixed_direction_text() {
        let text: Vec<u32> = "abc אבג".chars().map(u32::from).collect();
        let (state, direction) =
            super::preprocess_bidi_paragraph(&text, super::BidiDirection::Neutral);

        assert_eq!(direction, super::BidiDirection::LeftToRight);
        assert_eq!(super::bidi_embedding_levels(&state), [0, 0, 0, 0, 1, 1, 1]);

        let reordered = super::reorder_bidi_line(&text, &state);
        let reordered: String = reordered.into_iter().filter_map(char::from_u32).collect();
        assert_eq!(reordered, "abc גבא");
    }
}
