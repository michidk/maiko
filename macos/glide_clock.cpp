#include <sys/sysctl.h>
#include <stdint.h>

float ClockFrequency(void) {
    uint64_t hz = 0;
    size_t size = sizeof hz;
    /* Apple Silicon need not expose hw.cpufrequency. Zero means unknown,
       matching OpenGLide's existing inventory-only Linux failure result. */
    if (sysctlbyname("hw.cpufrequency", &hz, &size, nullptr, 0) || size != sizeof hz)
        return 0;
    return static_cast<float>(hz);
}
