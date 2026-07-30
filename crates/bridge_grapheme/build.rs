// Copyright 2026 the Tectonic Project
// Licensed under the MIT License.

//! Build the line-breaking subset of the vendored libgrapheme source.

use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn build_table(source: &Path, out_dir: &Path, name: &str) {
    let host = env::var("HOST").unwrap();
    let generator = out_dir.join(if cfg!(windows) {
        format!("grapheme-gen-{name}.exe")
    } else {
        format!("grapheme-gen-{name}")
    });
    let compiler = cc::Build::new().host(&host).target(&host).get_compiler();
    let mut command = compiler.to_command();

    command
        .arg(source.join(format!("gen/{name}.c")))
        .arg(source.join("gen/util.c"))
        .arg("-I")
        .arg(source)
        .arg("-O2")
        .arg("-o")
        .arg(&generator);

    let status = command
        .status()
        .expect("failed to launch the libgrapheme table compiler");
    assert!(
        status.success(),
        "failed to compile libgrapheme's table generator"
    );

    let table = File::create(out_dir.join(format!("{name}.h")))
        .expect("failed to create generated libgrapheme table");
    let status = Command::new(generator)
        .current_dir(source)
        .stdout(Stdio::from(table))
        .status()
        .expect("failed to launch the libgrapheme table generator");
    assert!(status.success(), "failed to generate libgrapheme's table");
}

fn stage_source(source: &Path, out_dir: &Path, name: &str) -> PathBuf {
    let mut upstream_source = fs::read_to_string(source.join(format!("src/{name}.c")))
        .expect("failed to read libgrapheme source")
        .replace(
            &format!("#include \"../gen/{name}.h\""),
            &format!("#include \"{name}.h\""),
        )
        .replace("#include \"../grapheme.h\"", "#include \"grapheme.h\"");

    if name == "bidirectional" {
        upstream_source.push_str(
            r#"

int tectonic_grapheme_bidi_resolve_direction(
	const uint_least32_t *, size_t, int);

int
tectonic_grapheme_bidi_resolve_direction(const uint_least32_t *src,
                                         size_t srclen, int fallback)
{
	size_t i;
	int_least8_t isolate_level = 0;

	for (i = 0; i < srclen; i++) {
		enum bidi_property prop = get_bidi_property(src[i]);

		if ((prop == BIDI_PROP_LRI || prop == BIDI_PROP_RLI ||
		     prop == BIDI_PROP_FSI) &&
		    isolate_level < MAX_DEPTH) {
			isolate_level++;
		} else if (prop == BIDI_PROP_PDI && isolate_level > 0) {
			isolate_level--;
		}

		if (isolate_level > 0) {
			continue;
		}
		if (prop == BIDI_PROP_L) {
			return GRAPHEME_BIDIRECTIONAL_DIRECTION_LTR;
		} else if (prop == BIDI_PROP_AL || prop == BIDI_PROP_R) {
			return GRAPHEME_BIDIRECTIONAL_DIRECTION_RTL;
		}
	}

	return fallback;
}
"#,
        );
    }

    let staged_source = out_dir.join(format!("{name}.c"));
    fs::write(&staged_source, upstream_source).expect("failed to stage libgrapheme source");
    staged_source
}

fn main() {
    let source = PathBuf::from("libgrapheme");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let include_dir = source
        .canonicalize()
        .expect("failed to resolve libgrapheme include directory");

    build_table(&source, &out_dir, "line");
    build_table(&source, &out_dir, "bidirectional");
    build_table(&source, &out_dir, "character");

    // Keep the upstream source unmodified while redirecting generated-header
    // includes to Cargo's output directory.
    let line_source = stage_source(&source, &out_dir, "line");
    let bidi_source = stage_source(&source, &out_dir, "bidirectional");
    let character_source = stage_source(&source, &out_dir, "character");

    let mut build = cc::Build::new();
    build
        .define("_ISOC99_SOURCE", None)
        .include(&source)
        .include(source.join("src"))
        .include(&out_dir)
        .flag_if_supported("-ffunction-sections")
        .flag_if_supported("-fdata-sections")
        .file(line_source)
        .file(bidi_source)
        .file(character_source)
        .file(source.join("src/utf8.c"))
        .file(source.join("src/util.c"))
        .compile("grapheme");

    println!("cargo:include_path={}", include_dir.display());
    println!("cargo:rerun-if-changed=libgrapheme");
}
