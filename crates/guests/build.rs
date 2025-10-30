// use std::{collections::HashMap, env, path::PathBuf};

// use risc0_build::{DockerOptionsBuilder, GuestOptionsBuilder, embed_methods_with_options};
// fn main() {
//     // Builds can be made deterministic, and thereby reproducible, by using Docker to build the
//     // guest. Check the RISC0_USE_DOCKER variable and use Docker to build the guest if set.
//     println!("cargo:rerun-if-env-changed=RISC0_USE_DOCKER");
//     println!("cargo:rerun-if-changed=build.rs");
//     let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
//     let mut builder = GuestOptionsBuilder::default();
//     if env::var("RISC0_USE_DOCKER").is_ok() {
//         let docker_options = DockerOptionsBuilder::default()
//             .root_dir(manifest_dir.join("../../.."))
//             .build()
//             .unwrap();
//         builder.use_docker(docker_options);
//     }
//     let guest_options = builder.build().unwrap();

//     // Generate Rust source files for the methods crate.
//     embed_methods_with_options(HashMap::from([
//         ("ordergen-loop", guest_options.clone()),
//         ("bento-sample", guest_options.clone()),
//         // ("identity", guest_options),
//     ]));
// }
fn main() {
    risc0_build::embed_methods();
}
