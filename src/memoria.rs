//! Detalles de memoria para que Solar ocupe poco aunque abras carpetas
//! enormes.

/// Al empezar: limita las "arenas" de malloc de glibc. Cada hilo de trabajo
/// (listar, copiar…) podía quedarse con su propia arena y retener memoria
/// que ya nadie usa. Con dos alcanza de sobra para Solar.
pub fn ajustar() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        const M_ARENA_MAX: i32 = -8;
        mallopt(M_ARENA_MAX, 2);
    }
}

/// Suelta algo grande (el listado de una carpeta de 20 000 archivos) en otro
/// hilo, para que la ventana no se trabe liberándolo, y le devuelve al
/// sistema la memoria que quedó libre.
pub fn soltar_aparte<T: Send + 'static>(valor: T, grande: bool) {
    if !grande {
        drop(valor);
        return;
    }
    let r = std::thread::Builder::new().name("solar-soltar".into()).spawn(move || {
        drop(valor);
        devolver_memoria();
    });
    if let Err(e) = r {
        // Sin hilo: el valor se suelta aquí mismo al fallar el spawn
        drop(e);
    }
}

/// Devuelve al sistema la memoria libre que malloc tenía guardada.
pub fn devolver_memoria() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        malloc_trim(0);
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe extern "C" {
    fn mallopt(param: i32, value: i32) -> i32;
    fn malloc_trim(pad: usize) -> i32;
}
