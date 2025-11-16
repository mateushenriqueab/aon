{
  "targets": [
    {
      "target_name": "aon_binding",
      "sources": [
        "aon_binding.c"
      ],

      "include_dirs": [
        "<!(node -p \"require('node-addon-api').include\")"
      ],

      "cflags!": [ "-fno-exceptions" ],
      "cflags_cc!": [ "-fno-exceptions" ],

      "libraries": [
        "-L/root/aon_rust_core_with_ffi/aon_core/target/release",
        "-laon_core"
      ],

      "ldflags": [
        "-Wl,-rpath,'$$ORIGIN'"
      ],

      "conditions": [
        [ 'OS=="mac"', {
          "xcode_settings": {
            "OTHER_LDFLAGS": [
              "-Wl,-rpath,@loader_path/"
            ]
          }
        }]
      ]
    }
  ]
}
