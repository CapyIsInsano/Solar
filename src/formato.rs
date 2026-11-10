//! Formatos en español: tamaños, fechas, permisos y texto para buscar.

use crate::ffi;
use std::time::{SystemTime, UNIX_EPOCH};

/// 1 234 567 -> "1,2 MB" (unidades decimales, como Nautilus y Dolphin).
pub fn tamano(bytes: u64) -> String {
    const UNIDADES: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
    if bytes < 1000 {
        return if bytes == 1 { "1 byte".into() } else { format!("{bytes} bytes") };
    }
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNIDADES.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    let texto = if v >= 100.0 { format!("{v:.0}") } else { format!("{v:.1}") };
    format!("{} {}", texto.replace('.', ","), UNIDADES[u])
}

pub fn elementos(n: usize) -> String {
    if n == 1 { "1 elemento".into() } else { format!("{} elementos", miles(n as u64)) }
}

/// 12345 -> "12 345"
pub fn miles(n: u64) -> String {
    let s = n.to_string();
    let mut r = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            r.push('\u{202F}');
        }
        r.push(c);
    }
    r
}

#[derive(Clone, Copy)]
struct Fecha {
    anio: i32,
    mes: i32,
    dia: i32,
    hora: i32,
    minuto: i32,
    semana: i32,
}

fn local(instante: i64) -> Fecha {
    let mut c = [0i32; 7];
    unsafe { ffi::solar_fecha_local(instante, c.as_mut_ptr()) };
    Fecha { anio: c[0], mes: c[1], dia: c[2], hora: c[3], minuto: c[4], semana: c[6] }
}

pub fn ahora() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

const MESES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
const MESES_LARGOS: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre",
    "noviembre", "diciembre",
];
const DIAS: [&str; 7] = ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"];

/// "hoy 14:32", "ayer 09:10", "8 nov 14:32" (este año) o "8 nov 2024".
pub fn fecha(instante: i64) -> String {
    if instante <= 0 {
        return "—".into();
    }
    let f = local(instante);
    let hoy = local(ahora());
    let ayer = local(ahora() - 86_400);
    let mismo = |a: &Fecha, b: &Fecha| a.anio == b.anio && a.mes == b.mes && a.dia == b.dia;
    if mismo(&f, &hoy) {
        format!("hoy {:02}:{:02}", f.hora, f.minuto)
    } else if mismo(&f, &ayer) {
        format!("ayer {:02}:{:02}", f.hora, f.minuto)
    } else if f.anio == hoy.anio {
        format!("{} {} {:02}:{:02}", f.dia, MESES[(f.mes - 1).clamp(0, 11) as usize], f.hora, f.minuto)
    } else {
        format!("{} {} {}", f.dia, MESES[(f.mes - 1).clamp(0, 11) as usize], f.anio)
    }
}

/// "domingo, 8 de noviembre de 2026, 14:32"
pub fn fecha_completa(instante: i64) -> String {
    if instante <= 0 {
        return "—".into();
    }
    let f = local(instante);
    format!(
        "{}, {} de {} de {}, {:02}:{:02}",
        DIAS[f.semana.clamp(0, 6) as usize],
        f.dia,
        MESES_LARGOS[(f.mes - 1).clamp(0, 11) as usize],
        f.anio,
        f.hora,
        f.minuto
    )
}

/// 0o100755 -> "rwxr-xr-x"
pub fn permisos(modo: u32) -> String {
    let mut s = String::with_capacity(9);
    for desplazamiento in [6, 3, 0] {
        let b = (modo >> desplazamiento) & 7;
        s.push(if b & 4 != 0 { 'r' } else { '-' });
        s.push(if b & 2 != 0 { 'w' } else { '-' });
        s.push(if b & 1 != 0 { 'x' } else { '-' });
    }
    s
}

/// Minúsculas y sin acentos, para buscar ("Música" -> "musica").
/// Como `plegar`, pero devuelve un texto vacío (sin reservar memoria) si el
/// nombre ya está plegado, que es lo más común (minúsculas sin acentos).
pub fn plegar_si_hace_falta(s: &str) -> String {
    if s.bytes().all(|b| b.is_ascii() && !b.is_ascii_uppercase()) {
        String::new()
    } else {
        plegar(s)
    }
}

pub fn plegar(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ē' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            'ý' | 'ÿ' => 'y',
            otro => otro,
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn plegado_solo_cuando_hace_falta() {
        assert_eq!(plegar_si_hace_falta("archivo_01.txt"), "");
        assert_eq!(plegar_si_hace_falta("Música"), "musica");
        assert_eq!(plegar_si_hace_falta("README"), "readme");
    }

    #[test]
    fn tamanos() {
        assert_eq!(tamano(0), "0 bytes");
        assert_eq!(tamano(1), "1 byte");
        assert_eq!(tamano(1_234), "1,2 kB");
        assert_eq!(tamano(15_600_000_000), "15,6 GB");
        assert_eq!(tamano(512_000_000_000), "512 GB");
    }

    #[test]
    fn textos() {
        assert_eq!(plegar("Música Ñandú"), "musica nandu");
        assert_eq!(permisos(0o100755), "rwxr-xr-x");
        assert_eq!(elementos(1), "1 elemento");
        assert_eq!(miles(12345), "12\u{202F}345");
    }
}
