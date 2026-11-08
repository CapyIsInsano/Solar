//! ☀ Solar — script de compilación
//!
//! Compila el núcleo en C (nucleo/) como biblioteca estática y lo enlaza con
//! la interfaz en Rust. Lua 5.4 viene incluido (feature `lua-vendorizado`,
//! por defecto); con `--no-default-features --features lua-sistema` se usa
//! el Lua de tu distro vía pkg-config.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=nucleo/src");
    println!("cargo:rerun-if-changed=nucleo/include");
    println!("cargo:rerun-if-changed=build.rs");

    let objetivo = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if objetivo != "linux" {
        panic!(
            "Solar v0.1.0-alpha solo funciona en Linux por ahora \
             (el núcleo usa inotify, /proc y /sys). ¡Los ports llegarán!"
        );
    }

    let mut nucleo = cc::Build::new();
    nucleo
        .std("c11")
        .define("_GNU_SOURCE", None)
        .include("nucleo/include")
        .warnings(true)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-fstack-protector-strong");

    let mut fuentes: Vec<_> = fs::read_dir("nucleo/src")
        .expect("no se encontró nucleo/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect();
    fuentes.sort();
    for archivo in &fuentes {
        nucleo.file(archivo);
    }

    // Primero se compila el núcleo y después se anuncia Lua, para que el
    // enlazador vea las bibliotecas en el orden correcto.
    let lua = preparar_lua(&mut nucleo);
    nucleo.compile("solar_nucleo");
    lua.anunciar();
}

/// Cómo enlazar Lua (se anuncia a cargo después de compilar el núcleo).
struct Lua {
    lineas: Vec<String>,
}

impl Lua {
    fn anunciar(&self) {
        for l in &self.lineas {
            println!("{l}");
        }
    }
}

#[cfg(feature = "lua-sistema")]
fn preparar_lua(nucleo: &mut cc::Build) -> Lua {
    let nombres = ["lua5.4", "lua54", "lua-5.4", "lua"];
    let mut ultimo_error = String::new();
    for nombre in nombres {
        match pkg_config::Config::new()
            .atleast_version("5.3")
            .cargo_metadata(false)
            .probe(nombre)
        {
            Ok(lib) => {
                for inc in &lib.include_paths {
                    nucleo.include(inc);
                }
                let mut lineas = Vec::new();
                for dir in &lib.link_paths {
                    lineas.push(format!("cargo:rustc-link-search=native={}", dir.display()));
                }
                for l in &lib.libs {
                    lineas.push(format!("cargo:rustc-link-lib={l}"));
                }
                return Lua { lineas };
            }
            Err(e) => ultimo_error = e.to_string(),
        }
    }
    panic!(
        "No se encontró Lua 5.4 con pkg-config ({ultimo_error}).\n\
         Instálalo (Gentoo: dev-lang/lua:5.4, Arch: lua, Debian: liblua5.4-dev)\n\
         o compila sin --features lua-sistema para usar el Lua incluido."
    );
}

#[cfg(all(feature = "lua-vendorizado", not(feature = "lua-sistema")))]
fn preparar_lua(nucleo: &mut cc::Build) -> Lua {
    let salida = env::var("OUT_DIR").expect("OUT_DIR");
    let artefactos = lua_src::Build::new()
        .out_dir(Path::new(&salida).join("lua"))
        .build(lua_src::Lua54);
    nucleo.include(artefactos.include_dir());
    let mut lineas = vec![
        format!("cargo:rustc-link-search=native={}", artefactos.lib_dir().display()),
    ];
    for lib in artefactos.libs() {
        lineas.push(format!("cargo:rustc-link-lib=static={lib}"));
    }
    lineas.push("cargo:rustc-link-lib=m".to_string());
    Lua { lineas }
}

#[cfg(not(any(feature = "lua-vendorizado", feature = "lua-sistema")))]
fn preparar_lua(_: &mut cc::Build) -> Lua {
    compile_error!("Activa la feature `lua-vendorizado` (por defecto) o `lua-sistema`.");
}
