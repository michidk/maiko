#ifndef RG_INPUT_H
#define RG_INPUT_H
#if defined(_WIN32) && !defined(RG_USE_SDL)
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
int rg_mouse_service(uint32_t registers[7]);
int rg_input_pump(void);
int rg_input_take_scan(void);
#ifdef __cplusplus
}
#endif
#endif
