/*
 * Independent ABI probe compiled against the checked-in Korg SDK headers.
 * Korg declarations used here are licensed under the BSD 3-Clause license in
 * external/logue-sdk/LICENSE.
 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

/* unit_genericfx.h includes DSP helper APIs that are unrelated to its ABI
 * declarations and only compile for ARM. Suppress those two transitive headers
 * while still compiling the authoritative unit_genericfx.h itself. */
#define __osc_api_h
#define __fx_api_h
#include "unit_genericfx.h"

#define LAYOUT(type, label)                                                   \
  printf("LAYOUT.%s.size=%zu\n", label, sizeof(type));                       \
  printf("LAYOUT.%s.align=%zu\n", label, _Alignof(type))
#define OFFSET(type, field, label)                                            \
  printf("OFFSET.%s=%zu\n", label, offsetof(type, field))
#define CONST_U(label, value) printf("CONST.%s=%u\n", label, (unsigned)(value))
#define CONST_I(label, value) printf("CONST.%s=%d\n", label, (int)(value))
#define SIGNATURE(type, expected, label)                                      \
  printf("SIGNATURE.%s=%d\n", label,                                        \
         __builtin_types_compatible_p(type, expected))

static const genericfx_unit_header_t dummy_header = {
    .common = {
        .header_size = sizeof(genericfx_unit_header_t),
        .target = UNIT_TARGET_PLATFORM | k_unit_module_genericfx,
        .api = UNIT_API_VERSION,
        .dev_id = 0,
        .unit_id = 0,
        .version = 0x00010000U,
        .name = "dummy",
        .num_params = 4,
        .params = {
            {0, 1023, 0, 0, k_unit_param_type_none, 0, 0, 0, {"PARAM1"}},
            {0, 1023, 0, 0, k_unit_param_type_none, 0, 0, 0, {"PARAM2"}},
            {-1000, 1000, 0, 0, k_unit_param_type_drywet, 1, 1, 0, {"DEPTH"}},
            {0, 3, 0, 1, k_unit_param_type_strings, 0, 0, 0, {"PARAM4"}},
            {0, 0, 0, 0, k_unit_param_type_none, 0, 0, 0, {""}},
            {0, 0, 0, 0, k_unit_param_type_none, 0, 0, 0, {""}},
            {0, 0, 0, 0, k_unit_param_type_none, 0, 0, 0, {""}},
            {0, 0, 0, 0, k_unit_param_type_none, 0, 0, 0, {""}},
        },
    },
    .default_mappings = {
        {k_genericfx_param_assign_x, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 1023, 256},
        {k_genericfx_param_assign_y, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 512, 1023, 512},
        {k_genericfx_param_assign_depth, k_genericfx_curve_exp,
         k_genericfx_curve_bipolar, -1000, 1000, 0},
        {k_genericfx_param_assign_none, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 3, 1},
        {k_genericfx_param_assign_none, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 0, 0},
        {k_genericfx_param_assign_none, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 0, 0},
        {k_genericfx_param_assign_none, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 0, 0},
        {k_genericfx_param_assign_none, k_genericfx_curve_linear,
         k_genericfx_curve_unipolar, 0, 0, 0},
    },
};

int main(void) {
  const unsigned char *bytes = (const unsigned char *)&dummy_header;
  size_t i;

  LAYOUT(unit_runtime_hooks_t, "runtime_hooks");
  OFFSET(unit_runtime_hooks_t, runtime_context, "runtime_hooks.context");
  OFFSET(unit_runtime_hooks_t, sdram_alloc, "runtime_hooks.alloc");
  OFFSET(unit_runtime_hooks_t, sdram_free, "runtime_hooks.free");
  OFFSET(unit_runtime_hooks_t, sdram_avail, "runtime_hooks.avail");

  LAYOUT(unit_runtime_desc_t, "runtime_desc");
  OFFSET(unit_runtime_desc_t, target, "runtime_desc.target");
  OFFSET(unit_runtime_desc_t, api, "runtime_desc.api");
  OFFSET(unit_runtime_desc_t, samplerate, "runtime_desc.sample_rate");
  OFFSET(unit_runtime_desc_t, frames_per_buffer, "runtime_desc.frames");
  OFFSET(unit_runtime_desc_t, input_channels, "runtime_desc.inputs");
  OFFSET(unit_runtime_desc_t, output_channels, "runtime_desc.outputs");
  OFFSET(unit_runtime_desc_t, hooks, "runtime_desc.hooks");

  LAYOUT(unit_runtime_genericfx_context_t, "genericfx_context");
  OFFSET(unit_runtime_genericfx_context_t, touch_area_width,
         "genericfx_context.width");
  OFFSET(unit_runtime_genericfx_context_t, touch_area_height,
         "genericfx_context.height");
  OFFSET(unit_runtime_genericfx_context_t, get_raw_input,
         "genericfx_context.raw_input");

  LAYOUT(unit_param_t, "unit_param");
  OFFSET(unit_param_t, min, "unit_param.min");
  OFFSET(unit_param_t, max, "unit_param.max");
  OFFSET(unit_param_t, center, "unit_param.center");
  OFFSET(unit_param_t, init, "unit_param.init");
  OFFSET(unit_param_t, type, "unit_param.type");
  printf("OFFSET.unit_param.format=%zu\n", offsetof(unit_param_t, name) - 1);
  OFFSET(unit_param_t, name, "unit_param.name");

  LAYOUT(unit_header_t, "unit_header");
  OFFSET(unit_header_t, header_size, "unit_header.header_size");
  OFFSET(unit_header_t, target, "unit_header.target");
  OFFSET(unit_header_t, api, "unit_header.api");
  OFFSET(unit_header_t, dev_id, "unit_header.dev_id");
  OFFSET(unit_header_t, unit_id, "unit_header.unit_id");
  OFFSET(unit_header_t, version, "unit_header.version");
  OFFSET(unit_header_t, name, "unit_header.name");
  OFFSET(unit_header_t, reserved0, "unit_header.reserved0");
  OFFSET(unit_header_t, reserved1, "unit_header.reserved1");
  OFFSET(unit_header_t, num_params, "unit_header.num_params");
  OFFSET(unit_header_t, params, "unit_header.params");

  LAYOUT(genericfx_param_mapping_t, "mapping");
  OFFSET(genericfx_param_mapping_t, assign, "mapping.assign");
  printf("OFFSET.mapping.curve=%zu\n",
         offsetof(genericfx_param_mapping_t, min) - 1);
  OFFSET(genericfx_param_mapping_t, min, "mapping.min");
  OFFSET(genericfx_param_mapping_t, max, "mapping.max");
  OFFSET(genericfx_param_mapping_t, value, "mapping.value");

  LAYOUT(genericfx_unit_header_t, "generic_header");
  OFFSET(genericfx_unit_header_t, common, "generic_header.common");
  OFFSET(genericfx_unit_header_t, default_mappings,
         "generic_header.mappings");

  CONST_U("UNIT_MODULE_GLOBAL", k_unit_module_global);
  CONST_U("UNIT_MODULE_MODFX", k_unit_module_modfx);
  CONST_U("UNIT_MODULE_DELFX", k_unit_module_delfx);
  CONST_U("UNIT_MODULE_REVFX", k_unit_module_revfx);
  CONST_U("UNIT_MODULE_OSC", k_unit_module_osc);
  CONST_U("UNIT_MODULE_SYNTH", k_unit_module_synth);
  CONST_U("UNIT_MODULE_MASTERFX", k_unit_module_masterfx);
  CONST_U("UNIT_MODULE_GENERICFX", k_unit_module_genericfx);
  CONST_U("NUM_UNIT_MODULES", k_num_unit_modules);
  CONST_U("TARGET_NTS3", k_unit_target_nts3_kaoss);
  CONST_U("TARGET_NTS3_GLOBAL", k_unit_target_nts3_kaoss_global);
  CONST_U("TARGET_NTS3_GENERICFX", k_unit_target_nts3_kaoss_genericfx);
  CONST_U("TARGET_PLATFORM", UNIT_TARGET_PLATFORM);
  CONST_U("TARGET_PLATFORM_MASK", UNIT_TARGET_PLATFORM_MASK);
  CONST_U("TARGET_MODULE_MASK", UNIT_TARGET_MODULE_MASK);
  CONST_U("API_1_0_0", k_unit_api_1_0_0);
  CONST_U("API_1_1_0", k_unit_api_1_1_0);
  CONST_U("API_2_0_0", k_unit_api_2_0_0);
  CONST_U("API_VERSION", UNIT_API_VERSION);
  CONST_U("API_MAJOR_MASK", UNIT_API_MAJOR_MASK);
  CONST_U("API_MINOR_MASK", UNIT_API_MINOR_MASK);
  CONST_U("API_PATCH_MASK", UNIT_API_PATCH_MASK);
  CONST_U("MAX_PARAMS", UNIT_MAX_PARAM_COUNT);
  CONST_U("GENERICFX_MAX_PARAMS", UNIT_GENERICFX_MAX_PARAM_COUNT);
  CONST_U("PARAM_NAME_LEN", UNIT_PARAM_NAME_LEN);
  CONST_U("PARAM_NAME_SIZE", UNIT_PARAM_NAME_SIZE);
  CONST_U("UNIT_NAME_LEN", UNIT_NAME_LEN);
  CONST_U("UNIT_NAME_SIZE", UNIT_NAME_SIZE);
  CONST_U("GENERICFX_FIXED_IDS", k_num_unit_genericfx_fixed_param_id);

  CONST_U("PARAM_NONE", k_unit_param_type_none);
  CONST_U("PARAM_PERCENT", k_unit_param_type_percent);
  CONST_U("PARAM_DB", k_unit_param_type_db);
  CONST_U("PARAM_CENTS", k_unit_param_type_cents);
  CONST_U("PARAM_SEMI", k_unit_param_type_semi);
  CONST_U("PARAM_OCT", k_unit_param_type_oct);
  CONST_U("PARAM_HERTZ", k_unit_param_type_hertz);
  CONST_U("PARAM_KHERTZ", k_unit_param_type_khertz);
  CONST_U("PARAM_BPM", k_unit_param_type_bpm);
  CONST_U("PARAM_MSEC", k_unit_param_type_msec);
  CONST_U("PARAM_SEC", k_unit_param_type_sec);
  CONST_U("PARAM_ENUM", k_unit_param_type_enum);
  CONST_U("PARAM_STRINGS", k_unit_param_type_strings);
  CONST_U("PARAM_RESERVED0", k_unit_param_type_reserved0);
  CONST_U("PARAM_DRYWET", k_unit_param_type_drywet);
  CONST_U("PARAM_PAN", k_unit_param_type_pan);
  CONST_U("PARAM_SPREAD", k_unit_param_type_spread);
  CONST_U("PARAM_ONOFF", k_unit_param_type_onoff);
  CONST_U("PARAM_MIDI_NOTE", k_unit_param_type_midi_note);
  CONST_U("PARAM_TYPE_COUNT", k_unit_param_type_count);
  CONST_U("FRAC_FIXED", k_unit_param_frac_mode_fixed);
  CONST_U("FRAC_DECIMAL", k_unit_param_frac_mode_decimal);

  CONST_U("ASSIGN_NONE", k_genericfx_param_assign_none);
  CONST_U("ASSIGN_X", k_genericfx_param_assign_x);
  CONST_U("ASSIGN_Y", k_genericfx_param_assign_y);
  CONST_U("ASSIGN_DEPTH", k_genericfx_param_assign_depth);
  CONST_U("ASSIGN_COUNT", k_num_genericfx_param_assign);
  CONST_U("CURVE_LINEAR", k_genericfx_curve_linear);
  CONST_U("CURVE_EXP", k_genericfx_curve_exp);
  CONST_U("CURVE_LOG", k_genericfx_curve_log);
  CONST_U("CURVE_TOGGLE", k_genericfx_curve_toggle);
  CONST_U("CURVE_MINCLIP", k_genericfx_curve_minclip);
  CONST_U("CURVE_MAXCLIP", k_genericfx_curve_maxclip);
  CONST_U("CURVE_COUNT", k_num_genericfx_curve);
  CONST_U("CURVE_UNIPOLAR", k_genericfx_curve_unipolar);
  CONST_U("CURVE_BIPOLAR", k_genericfx_curve_bipolar);
  CONST_U("CURVE_POLARITY_COUNT", k_num_genericfx_curve_polarity);

  CONST_U("TOUCH_BEGAN", k_unit_touch_phase_began);
  CONST_U("TOUCH_MOVED", k_unit_touch_phase_moved);
  CONST_U("TOUCH_ENDED", k_unit_touch_phase_ended);
  CONST_U("TOUCH_STATIONARY", k_unit_touch_phase_stationary);
  CONST_U("TOUCH_CANCELLED", k_unit_touch_phase_cancelled);
  CONST_U("TOUCH_COUNT", k_num_unit_touch_phases);

  CONST_I("ERR_NONE", k_unit_err_none);
  CONST_I("ERR_TARGET", k_unit_err_target);
  CONST_I("ERR_API_VERSION", k_unit_err_api_version);
  CONST_I("ERR_SAMPLERATE", k_unit_err_samplerate);
  CONST_I("ERR_GEOMETRY", k_unit_err_geometry);
  CONST_I("ERR_MEMORY", k_unit_err_memory);
  CONST_I("ERR_UNDEF", k_unit_err_undef);

  SIGNATURE(unit_runtime_sdram_alloc_ptr, uint8_t *(*)(size_t), "sdram_alloc");
  SIGNATURE(unit_runtime_sdram_free_ptr, void (*)(const uint8_t *), "sdram_free");
  SIGNATURE(unit_runtime_sdram_avail_ptr, size_t (*)(void), "sdram_avail");
  SIGNATURE(unit_runtime_genericfx_get_raw_input_ptr, const float *(*)(void),
            "raw_input");
  SIGNATURE(unit_init_func, int8_t (*)(const unit_runtime_desc_t *), "unit_init");
  SIGNATURE(unit_teardown_func, void (*)(void), "unit_teardown");
  SIGNATURE(unit_reset_func, void (*)(void), "unit_reset");
  SIGNATURE(unit_resume_func, void (*)(void), "unit_resume");
  SIGNATURE(unit_suspend_func, void (*)(void), "unit_suspend");
  SIGNATURE(unit_render_func, void (*)(const float *, float *, uint32_t),
            "unit_render");
  SIGNATURE(unit_get_param_value_func, int32_t (*)(uint8_t), "get_param");
  SIGNATURE(unit_get_param_str_value_func, const char *(*)(uint8_t, int32_t),
            "get_param_string");
  SIGNATURE(unit_set_param_value_func, void (*)(uint8_t, int32_t), "set_param");
  SIGNATURE(unit_set_tempo_func, void (*)(uint32_t), "set_tempo");
  SIGNATURE(unit_tempo_4ppqn_tick_func, void (*)(uint32_t), "tempo_tick");
  SIGNATURE(unit_touch_event_func, void (*)(uint8_t, uint8_t, uint32_t, uint32_t),
            "touch_event");

  printf("HEADER.bytes=");
  for (i = 0; i < sizeof(dummy_header); ++i) {
    printf("%02x", bytes[i]);
  }
  putchar('\n');
  return 0;
}
