// dlopen and dlsym for a static musl binary.
//
// The dlopen of a static musl is a stub that always fails, so winit and
// softbuffer find no X11 and no Wayland, and every window falls back to the
// terminal. The build links the libraries they open into the binary, and these
// two functions answer from a table of their symbols. musl declares dlopen and
// __dlsym weak, so these take their place, and its dlsym calls __dlsym.

#include <stddef.h>
#include <string.h>

struct dlshim_symbol {
    const char *name;
    void *address;
};

// Sorted by name. The build writes them in dltable.c.
extern const struct dlshim_symbol dlshim_symbols[];
extern const size_t dlshim_symbols_len;
extern const char *const dlshim_libraries[];
extern const size_t dlshim_libraries_len;

static char handle;

// Returns `true` if `file` names a library in the binary, `false` otherwise. A
// name matches with or without the directory and the version after `.so`.
static int linked(const char *file) {
    const char *base = strrchr(file, '/');
    base = base ? base + 1 : file;
    const char *so = strstr(base, ".so");
    size_t len = so ? (size_t)(so - base) : strlen(base);
    for (size_t i = 0; i < dlshim_libraries_len; i++) {
        if (strlen(dlshim_libraries[i]) == len && memcmp(dlshim_libraries[i], base, len) == 0) {
            return 1;
        }
    }
    return 0;
}

void *dlopen(const char *file, int mode) {
    (void)mode;
    if (file == NULL || linked(file)) {
        return &handle;
    }
    return NULL;
}

void *__dlsym(void *restrict p, const char *restrict name, void *restrict ra) {
    (void)p;
    (void)ra;
    size_t lo = 0, hi = dlshim_symbols_len;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int cmp = strcmp(name, dlshim_symbols[mid].name);
        if (cmp == 0) {
            return dlshim_symbols[mid].address;
        }
        if (cmp < 0) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    return NULL;
}
