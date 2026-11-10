//! Medidor de tiempos para buscar cuellos de botella.
//! Se activa con SOLAR_PERFIL=1 y cada 3 s escribe en la terminal cuánto
//! tardó en promedio cada parte del cuadro. Apagado no cuesta nada.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub fn activo() -> bool {
    static A: OnceLock<bool> = OnceLock::new();
    *A.get_or_init(|| std::env::var_os("SOLAR_PERFIL").is_some_and(|v| v != "0"))
}

struct Datos {
    suma: BTreeMap<&'static str, (Duration, u32)>,
    desde: Instant,
    cuadros: u32,
}

thread_local! {
    static DATOS: RefCell<Datos> = RefCell::new(Datos { suma: BTreeMap::new(), desde: Instant::now(), cuadros: 0 });
}

/// Mide lo que tarda `f` bajo el nombre `que`.
#[inline]
pub fn medir<R>(que: &'static str, f: impl FnOnce() -> R) -> R {
    if !activo() {
        return f();
    }
    let t = Instant::now();
    let r = f();
    let d = t.elapsed();
    DATOS.with(|x| {
        let mut x = x.borrow_mut();
        let e = x.suma.entry(que).or_default();
        e.0 += d;
        e.1 += 1;
    });
    r
}

/// Llamar una vez por cuadro.
pub fn cuadro() {
    if !activo() {
        return;
    }
    DATOS.with(|x| {
        let mut x = x.borrow_mut();
        x.cuadros += 1;
        if x.desde.elapsed() >= Duration::from_secs(3) {
            let n = x.cuadros.max(1);
            let partes: Vec<String> = x
                .suma
                .iter()
                .map(|(k, (d, _))| format!("{k} {:.2} ms", d.as_secs_f64() * 1000.0 / n as f64))
                .collect();
            eprintln!("[perfil] {n} cuadros · por cuadro: {}", partes.join(" · "));
            x.suma.clear();
            x.cuadros = 0;
            x.desde = Instant::now();
        }
    });
}

/// Para hilos de trabajo: escribe cuánto tardó algo puntual (si está activo).
pub fn reportar(que: &str, desde: Instant) {
    if activo() {
        eprintln!("[perfil] {que}: {:.1} ms", desde.elapsed().as_secs_f64() * 1000.0);
    }
}
