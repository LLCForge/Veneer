Extension data files. Every `*.json` file here is embedded at build time and available to
extension modules as `crate::embedded::EXTRA_DATA_FILES` (file stem, contents). Use one file per
topic (for example `gpu-params.json`, `speech-voices.json`) and document its source in ../SOURCES.md.
