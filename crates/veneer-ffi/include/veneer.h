/*
 * Veneer C ABI. Build with `cargo build -p veneer-ffi --release`; link veneer_ffi
 * (veneer_ffi.dll / libveneer_ffi.so / libveneer_ffi.dylib, or the static library).
 *
 * Every function returns a NUL-terminated UTF-8 JSON envelope, {"ok": value} or
 * {"error": "message"}, owned by the caller and released with veneer_free().
 * NULL is returned only when the envelope itself could not be allocated.
 * JSON arguments must be NUL-terminated UTF-8, at most 4 MiB.
 */
#ifndef VENEER_H
#define VENEER_H

#ifdef __cplusplus
extern "C" {
#endif

/* constraints_json may be NULL. ok: profile object. */
char *veneer_generate(const char *constraints_json);
/* ok: {valid, score, issues}. */
char *veneer_validate(const char *profile_json);
/* ok: init script string. */
char *veneer_init_script(const char *profile_json);
/* ok: [{name, value}] navigation request headers in browser order. */
char *veneer_navigation_headers(const char *profile_json);
/* ok: catalog object. */
char *veneer_catalog(void);
/* ok: collector expression string for a page evaluate call. */
char *veneer_collector_script(void);
/* ok: {valid, score, issues}. */
char *veneer_compare_observed(const char *profile_json, const char *observed_json);
/* Releases a string returned above; NULL is ignored. */
void veneer_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* VENEER_H */
