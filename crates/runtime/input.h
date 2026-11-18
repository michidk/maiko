#ifndef RG_INPUT_H
#define RG_INPUT_H
#if defined(_WIN32)
#include <windows.h>
typedef HWND RgWindow;
#else
typedef void *RgWindow;
#endif
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
RgWindow rg_input_window(unsigned width, unsigned height);
int rg_input_pump(void);
/* Tests only: one window event through the real input paths (see input.rs). */
int rg_input_test_event(unsigned kind, unsigned value);
#ifdef __cplusplus
}
#endif
#endif
