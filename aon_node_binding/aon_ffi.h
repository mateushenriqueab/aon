#ifndef AON_FFI_H
#define AON_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

// JSON -> AON (root schema required)
char* aon_json_to_aon(const char* json, const char* root_schema_name);

// AON -> JSON
char* aon_aon_to_json(const char* aon_text);

// Get last error message
const char* aon_last_error();

// Free string allocated by Rust
void aon_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // AON_FFI_H
