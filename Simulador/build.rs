use std::{env, path::PathBuf};

// Enlaza raylib y las bibliotecas de Windows que necesita
fn main() {
    println!("cargo:rerun-if-changed=vendor/raylib");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
            panic!("En Windows usa Rust MSVC x86_64; consulta el README.");
        }
        let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let lib = root.join("vendor/raylib/lib/x64");
        assert!(
            lib.join("raylib.lib").exists(),
            "Falta vendor/raylib/lib/x64/raylib.lib"
        );
        println!("cargo:rustc-link-search=native={}", lib.display());
        // raylib enlazado de forma estatica: el ejecutable no necesita DLL
        println!("cargo:rustc-link-lib=static=raylib");
        for system in ["winmm", "gdi32", "user32", "shell32", "opengl32"] {
            println!("cargo:rustc-link-lib={system}");
        }
    }
}
