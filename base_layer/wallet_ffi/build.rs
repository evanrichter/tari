// Copyright 2022. The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::{env, path::PathBuf};

use cbindgen::{Config, Language, ParseConfig, Style};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let output_file = PathBuf::from(&crate_dir)
        .join(format!("{}.h", package_name))
        .display()
        .to_string();

    let config = Config {
        language: Language::C,
        header: Some("// Copyright 2022. The Tari Project\n// SPDX-License-Identifier: BSD-3-Clause".to_string()),
        parse: ParseConfig {
            parse_deps: true,
            include: Some(vec![
                "tari_core".to_string(),
                "tari_common_types".to_string(),
                "tari_crypto".to_string(),
                "tari_p2p".to_string(),
                "tari_wallet".to_string(),
            ]),
            ..Default::default()
        },
        style: Style::Tag,
        ..Default::default()
    };

    cbindgen::generate_with_config(&crate_dir, config)
        .unwrap()
        .write_to_file(&output_file);
}

// /// Find the location of the `target/` directory. Note that this may be
// /// overridden by `cmake`, so we also need to check the `CARGO_TARGET_DIR`
// /// variable.
// fn target_dir() -> PathBuf {
//     if let Ok(target) = env::var("CARGO_TARGET_DIR") {
//         PathBuf::from(target)
//     } else {
//         PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target")
//     }
// }
