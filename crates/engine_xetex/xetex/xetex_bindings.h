#ifndef TECTONIC_ENGINE_XETEX_BINDGEN_H
#define TECTONIC_ENGINE_XETEX_BINDGEN_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * A serial number describing the detailed binary layout of the TeX "format
 * files" used by this crate. This number will occasionally increment,
 * indicating that the format file structure has changed. There is no provision
 * for partial forwards or backwards compatibility: if the number changes, you
 * need to regenerate your format files. If you’re generating format files, you
 * should munge this serial number in the filename, or something along those
 * lines, to make sure that when the engine is updated you don’t attempt to
 * reuse old files.
 */
#define FORMAT_SERIAL 33

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

typedef struct tt_xetex_profile_t {
  uint64_t setup_before_format_us;
  uint64_t format_load_us;
  uint64_t post_format_setup_us;
  uint64_t start_input_us;
  uint64_t main_control_us;
  uint64_t preamble_us;
  uint64_t body_us;
  uint64_t final_cleanup_us;
  uint64_t close_files_us;
  uint64_t cleanup_us;
  uint64_t total_us;
  uint64_t heap_capacity_before_first_font;
  uint64_t heap_live_before_first_font;
  uint64_t heap_free_before_first_font;
  uint64_t heap_arena_before_first_font;
  uint64_t heap_capacity_after_first_font;
  uint64_t heap_live_after_first_font;
  uint64_t heap_free_after_first_font;
  uint64_t heap_arena_after_first_font;
  uint64_t heap_capacity_after_latest_font;
  uint64_t heap_live_after_latest_font;
  uint64_t heap_free_after_latest_font;
  uint64_t heap_arena_after_latest_font;
  uint32_t checkpoint_count;
  uint32_t resident_resume;
  uint32_t loaded_font_count;
} tt_xetex_profile_t;

extern int tt_xetex_set_int_variable(const char *var_name, int value);

extern void tt_xetex_get_last_profile(tt_xetex_profile_t *profile);

extern int tt_engine_xetex_main(ttbc_state_t *api,
                                const char *dump_name,
                                const char *input_file_name,
                                uint64_t build_date);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* TECTONIC_ENGINE_XETEX_BINDGEN_H */
