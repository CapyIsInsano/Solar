//! Envoltorios seguros del núcleo en C.
//!
//! Todo lo "de sistema" (listar, copiar, papelera, USB, Lua…) lo hace el
//! núcleo; aquí solo se traducen los tipos de C a tipos de Rust y se
//! administra la memoria.

use crate::ffi;
use eframe::egui::Color32;
use std::sync::Arc;
use std::ffi::{CStr, CString, OsStr, c_char, c_int, c_void};
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr;

// ======================================================================
//  Errores y conversiones
// ======================================================================

#[derive(Debug, Clone)]
pub struct ErrorSolar {
    pub codigo: i32,
    pub mensaje: String,
}

impl fmt::Display for ErrorSolar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.mensaje)
    }
}

impl ErrorSolar {
    pub fn de_codigo(codigo: c_int) -> Self {
        Self { codigo, mensaje: texto_error(codigo) }
    }

    fn de_buffer(codigo: c_int, buf: &[c_char]) -> Self {
        let mensaje = texto_de_buffer(buf);
        if mensaje.is_empty() {
            Self::de_codigo(codigo)
        } else {
            Self { codigo, mensaje }
        }
    }

    pub fn nuevo(mensaje: impl Into<String>) -> Self {
        Self { codigo: libc_einval(), mensaje: mensaje.into() }
    }

    pub fn cancelado(&self) -> bool {
        self.codigo == 125 // ECANCELED en Linux
    }
}

const fn libc_einval() -> i32 {
    22
}

pub fn texto_error(codigo: i32) -> String {
    unsafe { CStr::from_ptr(ffi::solar_error_texto(codigo)) }
        .to_string_lossy()
        .into_owned()
}

fn resultado(codigo: c_int) -> Result<(), ErrorSolar> {
    if codigo == 0 { Ok(()) } else { Err(ErrorSolar::de_codigo(codigo)) }
}

pub fn cruta(p: &Path) -> CString {
    let bytes: Vec<u8> = p.as_os_str().as_bytes().iter().copied().filter(|b| *b != 0).collect();
    CString::new(bytes).unwrap_or_default()
}

pub fn ctexto(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// # Safety
/// `p` debe ser NULL o una cadena C válida.
unsafe fn texto_de(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// # Safety
/// `p` debe ser NULL o una cadena C válida.
unsafe fn ruta_de(p: *const c_char) -> PathBuf {
    if p.is_null() {
        return PathBuf::new();
    }
    PathBuf::from(OsStr::from_bytes(unsafe { CStr::from_ptr(p) }.to_bytes()))
}

fn texto_de_buffer(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    String::from_utf8_lossy(&bytes).trim().to_string()
}

fn ruta_de_buffer(buf: &[c_char]) -> PathBuf {
    let bytes: Vec<u8> = buf.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    PathBuf::from(OsStr::from_bytes(&bytes))
}

fn llamar_con_buffer(f: impl FnOnce(*mut c_char, usize) -> c_int) -> Option<PathBuf> {
    let mut buf = vec![0 as c_char; 4096];
    let r = f(buf.as_mut_ptr(), buf.len());
    (r == 0).then(|| ruta_de_buffer(&buf))
}

pub fn color_de(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub fn color_a(c: Color32) -> u32 {
    ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32
}

// ======================================================================
//  General
// ======================================================================

pub fn version() -> String {
    unsafe { texto_de(ffi::solar_version()) }
}

pub fn dir_config() -> PathBuf {
    llamar_con_buffer(|b, t| unsafe { ffi::solar_dir_config(b, t) })
        .unwrap_or_else(|| dir_hogar().join(".config/solar"))
}

pub fn dir_datos() -> PathBuf {
    llamar_con_buffer(|b, t| unsafe { ffi::solar_dir_datos(b, t) })
        .unwrap_or_else(|| dir_hogar().join(".local/share/solar"))
}

pub fn dir_hogar() -> PathBuf {
    llamar_con_buffer(|b, t| unsafe { ffi::solar_dir_hogar(b, t) }).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn expandir(ruta: &str) -> PathBuf {
    let c = ctexto(ruta.trim());
    llamar_con_buffer(|b, t| unsafe { ffi::solar_expandir_ruta(c.as_ptr(), b, t) })
        .unwrap_or_else(|| PathBuf::from(ruta))
}

/// "/home/capy/Juegos" -> "~/Juegos"
pub fn contraer(ruta: &Path) -> String {
    let hogar = dir_hogar();
    if let Ok(resto) = ruta.strip_prefix(&hogar)
        && hogar != Path::new("/")
    {
        if resto.as_os_str().is_empty() {
            return "~".into();
        }
        return format!("~/{}", resto.to_string_lossy());
    }
    ruta.to_string_lossy().into_owned()
}

// ======================================================================
//  Listado
// ======================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TipoEntrada {
    Archivo,
    Carpeta,
    EnlaceRoto,
    Especial,
}

impl TipoEntrada {
    fn de(t: u8) -> Self {
        match t {
            ffi::SOLAR_TIPO_CARPETA => Self::Carpeta,
            ffi::SOLAR_TIPO_ENLACE_ROTO => Self::EnlaceRoto,
            ffi::SOLAR_TIPO_ESPECIAL => Self::Especial,
            _ => Self::Archivo,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Categoria {
    Carpeta,
    Generico,
    Texto,
    Codigo,
    Imagen,
    Audio,
    Video,
    Comprimido,
    Pdf,
    Documento,
    HojaCalculo,
    Presentacion,
    Ejecutable,
    Fuente,
    Disco,
    Paquete,
}

impl Categoria {
    pub fn de(c: u8) -> Self {
        use Categoria::*;
        match c {
            0 => Carpeta,
            2 => Texto,
            3 => Codigo,
            4 => Imagen,
            5 => Audio,
            6 => Video,
            7 => Comprimido,
            8 => Pdf,
            9 => Documento,
            10 => HojaCalculo,
            11 => Presentacion,
            12 => Ejecutable,
            13 => Fuente,
            14 => Disco,
            15 => Paquete,
            _ => Generico,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orden {
    Nombre = 0,
    Tamano = 1,
    Fecha = 2,
    Tipo = 3,
}

impl Orden {
    pub fn de(v: i32) -> Self {
        match v {
            1 => Self::Tamano,
            2 => Self::Fecha,
            3 => Self::Tipo,
            _ => Self::Nombre,
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Self::Nombre => "Nombre",
            Self::Tamano => "Tamaño",
            Self::Fecha => "Fecha de modificación",
            Self::Tipo => "Tipo",
        }
    }

    pub const TODOS: [Orden; 4] = [Orden::Nombre, Orden::Tamano, Orden::Fecha, Orden::Tipo];
}

#[derive(Clone, Debug)]
pub struct Entrada {
    pub nombre: String,
    pub ruta: PathBuf,
    pub tamano: u64,
    pub modificado: i64,
    #[allow(dead_code)]
    pub modo: u32,
    pub tipo: TipoEntrada,
    pub categoria: Categoria,
    pub es_enlace: bool,
    pub es_oculto: bool,
    /// Compartida entre todos los archivos del mismo tipo ("Imagen PNG"…)
    pub descripcion: Arc<str>,
    pub estilo: Option<EstiloCarpeta>,
    /// Nombre sin mayúsculas ni acentos, para buscar. Vacío si es igual al
    /// nombre (así no se guarda dos veces): usa `plegado()`.
    pub plegado: String,
}

impl Entrada {
    pub fn es_carpeta(&self) -> bool {
        self.tipo == TipoEntrada::Carpeta
    }

    pub fn plegado(&self) -> &str {
        if self.plegado.is_empty() { &self.nombre } else { &self.plegado }
    }
}

pub fn describir_tipo(nombre: &str, categoria: u8, tipo: u8, es_enlace: bool) -> String {
    let c = ctexto(nombre);
    let mut buf = [0 as c_char; 128];
    unsafe {
        ffi::solar_describir_tipo(
            c.as_ptr(),
            categoria as c_int,
            tipo as c_int,
            es_enlace as c_int,
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    texto_de_buffer(&buf)
}

pub fn listar(dir: &Path, orden: Orden, descendente: bool, carpetas_primero: bool) -> Result<Vec<Entrada>, ErrorSolar> {
    let c = cruta(dir);
    let mut l = ffi::SolarListado { entradas: ptr::null_mut(), cantidad: 0 };
    let r = unsafe { ffi::solar_listar(c.as_ptr(), &mut l) };
    if r != 0 {
        unsafe { ffi::solar_listado_liberar(&mut l) };
        return Err(ErrorSolar::de_codigo(r));
    }
    unsafe { ffi::solar_ordenar(&mut l, orden as c_int, descendente as c_int, carpetas_primero as c_int) };
    let mut v = Vec::with_capacity(l.cantidad);
    // Miles de archivos comparten unas pocas descripciones ("Imagen PNG"…):
    // se guardan una sola vez y cada archivo apunta a la suya
    let mut descripciones: std::collections::HashMap<Box<str>, Arc<str>> = std::collections::HashMap::new();
    for i in 0..l.cantidad {
        let e = unsafe { &*l.entradas.add(i) };
        let bytes = unsafe { CStr::from_ptr(e.nombre) }.to_bytes();
        let nombre = String::from_utf8_lossy(bytes).into_owned();
        let mut desc = [0 as c_char; 128];
        unsafe {
            ffi::solar_describir_tipo(
                e.nombre,
                e.categoria as c_int,
                e.tipo as c_int,
                e.es_enlace as c_int,
                desc.as_mut_ptr(),
                desc.len(),
            )
        };
        let texto = unsafe { CStr::from_ptr(desc.as_ptr()) }.to_string_lossy();
        let descripcion = match descripciones.get(texto.as_ref()) {
            Some(d) => d.clone(),
            None => {
                let d: Arc<str> = Arc::from(texto.as_ref());
                descripciones.insert(texto.as_ref().into(), d.clone());
                d
            }
        };
        v.push(Entrada {
            plegado: crate::formato::plegar_si_hace_falta(&nombre),
            nombre,
            ruta: dir.join(OsStr::from_bytes(bytes)),
            tamano: e.tamano,
            modificado: e.modificado,
            modo: e.modo,
            tipo: TipoEntrada::de(e.tipo),
            categoria: Categoria::de(e.categoria),
            es_enlace: e.es_enlace != 0,
            es_oculto: e.es_oculto != 0,
            descripcion,
            estilo: None,
        });
    }
    unsafe { ffi::solar_listado_liberar(&mut l) };
    Ok(v)
}

// ======================================================================
//  Información
// ======================================================================

#[derive(Clone, Debug)]
pub struct Info {
    pub tamano: u64,
    pub modificado: i64,
    pub accedido: i64,
    pub creado: Option<i64>,
    pub modo: u32,
    pub tipo: TipoEntrada,
    pub categoria: Categoria,
    pub es_enlace: bool,
    pub propietario: String,
    pub grupo: String,
    pub destino_enlace: Option<String>,
    pub descripcion: String,
}

pub fn info(ruta: &Path) -> Result<Info, ErrorSolar> {
    let c = cruta(ruta);
    let mut i: ffi::SolarInfo = unsafe { std::mem::zeroed() };
    resultado(unsafe { ffi::solar_info(c.as_ptr(), &mut i) })?;
    let nombre = ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Info {
        tamano: i.tamano,
        modificado: i.modificado,
        accedido: i.accedido,
        creado: (i.creado > 0).then_some(i.creado),
        modo: i.modo,
        tipo: TipoEntrada::de(i.tipo),
        categoria: Categoria::de(i.categoria),
        es_enlace: i.es_enlace != 0,
        propietario: texto_de_buffer(&i.propietario),
        grupo: texto_de_buffer(&i.grupo),
        destino_enlace: (i.es_enlace != 0).then(|| texto_de_buffer(&i.destino_enlace)),
        descripcion: describir_tipo(&nombre, i.categoria, i.tipo, i.es_enlace != 0),
    })
}

pub fn cambiar_permisos(ruta: &Path, modo: u32) -> Result<(), ErrorSolar> {
    let c = cruta(ruta);
    resultado(unsafe { ffi::solar_cambiar_permisos(c.as_ptr(), modo) })
}

struct ContextoConteo<'a> {
    progreso: &'a mut dyn FnMut(u64, u64) -> bool,
}

unsafe extern "C" fn trampolin_conteo(
    datos: *mut c_void,
    bytes: u64,
    _bt: u64,
    vistos: u64,
    _et: u64,
    _actual: *const c_char,
) -> c_int {
    let ctx = unsafe { &mut *(datos as *mut ContextoConteo) };
    match catch_unwind(AssertUnwindSafe(|| (ctx.progreso)(bytes, vistos))) {
        Ok(true) => 0,
        _ => 1,
    }
}

/// Tamaño total (bytes, archivos, carpetas). `progreso` devuelve false para cancelar.
pub fn tamano_total(ruta: &Path, mut progreso: impl FnMut(u64, u64) -> bool) -> Result<(u64, u64, u64), ErrorSolar> {
    let c = cruta(ruta);
    let (mut b, mut a, mut d) = (0u64, 0u64, 0u64);
    let mut ctx = ContextoConteo { progreso: &mut progreso };
    let r = unsafe {
        ffi::solar_tamano_total(
            c.as_ptr(),
            &mut b,
            &mut a,
            &mut d,
            Some(trampolin_conteo),
            &mut ctx as *mut _ as *mut c_void,
        )
    };
    resultado(r).map(|_| (b, a, d))
}

// ======================================================================
//  Operaciones
// ======================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conflicto {
    Renombrar = 0,
    Reemplazar = 1,
    Omitir = 2,
}

#[derive(Clone, Default, Debug)]
pub struct Progreso {
    pub bytes_hechos: u64,
    pub bytes_total: u64,
    pub hechos: u64,
    pub total: u64,
    pub actual: String,
}

struct Contexto<'a> {
    progreso: &'a mut dyn FnMut(&Progreso) -> bool,
    resultado: &'a mut dyn FnMut(&Path, &Path),
}

unsafe extern "C" fn trampolin_progreso(
    datos: *mut c_void,
    bytes_hechos: u64,
    bytes_total: u64,
    hechos: u64,
    total: u64,
    actual: *const c_char,
) -> c_int {
    let ctx = unsafe { &mut *(datos as *mut Contexto) };
    let p = Progreso { bytes_hechos, bytes_total, hechos, total, actual: unsafe { texto_de(actual) } };
    match catch_unwind(AssertUnwindSafe(|| (ctx.progreso)(&p))) {
        Ok(true) => 0,
        _ => 1,
    }
}

unsafe extern "C" fn trampolin_resultado(datos: *mut c_void, origen: *const c_char, destino: *const c_char) {
    let ctx = unsafe { &mut *(datos as *mut Contexto) };
    let o = unsafe { ruta_de(origen) };
    let d = unsafe { ruta_de(destino) };
    let _ = catch_unwind(AssertUnwindSafe(|| (ctx.resultado)(&o, &d)));
}

fn con_opciones(
    conflicto: Conflicto,
    progreso: &mut dyn FnMut(&Progreso) -> bool,
    resultado_fn: &mut dyn FnMut(&Path, &Path),
    f: impl FnOnce(*const ffi::SolarOpciones) -> c_int,
) -> Result<(), ErrorSolar> {
    let mut error = vec![0 as c_char; 1024];
    let mut ctx = Contexto { progreso, resultado: resultado_fn };
    let op = ffi::SolarOpciones {
        conflicto: conflicto as i32,
        progreso: Some(trampolin_progreso),
        resultado: Some(trampolin_resultado),
        datos: &mut ctx as *mut _ as *mut c_void,
        error: error.as_mut_ptr(),
        error_tam: error.len(),
    };
    let r = f(&op);
    if r == 0 { Ok(()) } else { Err(ErrorSolar::de_buffer(r, &error)) }
}

fn arreglo(rutas: &[PathBuf]) -> (Vec<CString>, Vec<*const c_char>) {
    let c: Vec<CString> = rutas.iter().map(|r| cruta(r)).collect();
    let p = c.iter().map(|s| s.as_ptr()).collect();
    (c, p)
}

pub type FnProgreso<'a> = &'a mut dyn FnMut(&Progreso) -> bool;
pub type FnResultado<'a> = &'a mut dyn FnMut(&Path, &Path);

pub fn copiar(
    origenes: &[PathBuf],
    destino: &Path,
    conflicto: Conflicto,
    progreso: FnProgreso,
    resultado_fn: FnResultado,
) -> Result<(), ErrorSolar> {
    let (_c, p) = arreglo(origenes);
    let d = cruta(destino);
    con_opciones(conflicto, progreso, resultado_fn, |op| unsafe {
        ffi::solar_copiar(p.as_ptr(), p.len(), d.as_ptr(), op)
    })
}

pub fn mover(
    origenes: &[PathBuf],
    destino: &Path,
    conflicto: Conflicto,
    progreso: FnProgreso,
    resultado_fn: FnResultado,
) -> Result<(), ErrorSolar> {
    let (_c, p) = arreglo(origenes);
    let d = cruta(destino);
    con_opciones(conflicto, progreso, resultado_fn, |op| unsafe {
        ffi::solar_mover(p.as_ptr(), p.len(), d.as_ptr(), op)
    })
}

pub fn eliminar(rutas: &[PathBuf], progreso: FnProgreso, resultado_fn: FnResultado) -> Result<(), ErrorSolar> {
    let (_c, p) = arreglo(rutas);
    con_opciones(Conflicto::Renombrar, progreso, resultado_fn, |op| unsafe {
        ffi::solar_eliminar(p.as_ptr(), p.len(), op)
    })
}

pub fn papelera_enviar(rutas: &[PathBuf], progreso: FnProgreso, resultado_fn: FnResultado) -> Result<(), ErrorSolar> {
    let (_c, p) = arreglo(rutas);
    con_opciones(Conflicto::Renombrar, progreso, resultado_fn, |op| unsafe {
        ffi::solar_papelera_enviar(p.as_ptr(), p.len(), op)
    })
}

pub fn papelera_vaciar(progreso: FnProgreso) -> Result<(), ErrorSolar> {
    let mut nada = |_: &Path, _: &Path| {};
    con_opciones(Conflicto::Renombrar, progreso, &mut nada, |op| unsafe { ffi::solar_papelera_vaciar(op) })
}

pub fn renombrar(ruta: &Path, nombre_nuevo: &str) -> Result<PathBuf, ErrorSolar> {
    let c = cruta(ruta);
    let n = ctexto(nombre_nuevo);
    resultado(unsafe { ffi::solar_renombrar(c.as_ptr(), n.as_ptr()) })?;
    Ok(ruta.with_file_name(nombre_nuevo))
}

pub fn crear_carpeta(dir: &Path, nombre: &str) -> Result<PathBuf, ErrorSolar> {
    let d = cruta(dir);
    let n = ctexto(nombre);
    resultado(unsafe { ffi::solar_crear_carpeta(d.as_ptr(), n.as_ptr()) })?;
    Ok(dir.join(nombre))
}

pub fn crear_archivo(dir: &Path, nombre: &str) -> Result<PathBuf, ErrorSolar> {
    let d = cruta(dir);
    let n = ctexto(nombre);
    resultado(unsafe { ffi::solar_crear_archivo(d.as_ptr(), n.as_ptr()) })?;
    Ok(dir.join(nombre))
}

pub fn validar_nombre(nombre: &str) -> Result<(), ErrorSolar> {
    let n = ctexto(nombre);
    resultado(unsafe { ffi::solar_validar_nombre(n.as_ptr()) })
}

pub fn nombre_libre(dir: &Path, nombre: &str, es_copia: bool) -> String {
    let d = cruta(dir);
    let n = ctexto(nombre);
    let mut buf = [0 as c_char; 512];
    let r = unsafe { ffi::solar_nombre_libre(d.as_ptr(), n.as_ptr(), es_copia as c_int, buf.as_mut_ptr(), buf.len()) };
    if r == 0 { texto_de_buffer(&buf) } else { nombre.to_string() }
}

pub fn abrir(ruta: &Path) -> Result<(), ErrorSolar> {
    let c = cruta(ruta);
    resultado(unsafe { ffi::solar_abrir(c.as_ptr()) }).map_err(|e| {
        if e.codigo == 2 {
            ErrorSolar::nuevo("No se encontró «xdg-open». Instala xdg-utils para abrir archivos con su programa.")
        } else {
            e
        }
    })
}

pub fn abrir_con(comando: &str, rutas: &[PathBuf]) -> Result<(), ErrorSolar> {
    let c = ctexto(comando);
    let (_c, p) = arreglo(rutas);
    resultado(unsafe { ffi::solar_abrir_con(c.as_ptr(), p.as_ptr(), p.len()) }).map_err(|e| {
        if e.codigo == 2 { ErrorSolar::nuevo(format!("No se encontró el programa de «{comando}».")) } else { e }
    })
}

pub fn abrir_terminal(terminal: &str, dir: &Path) -> Result<(), ErrorSolar> {
    let t = ctexto(terminal);
    let d = cruta(dir);
    resultado(unsafe { ffi::solar_abrir_terminal(t.as_ptr(), d.as_ptr()) }).map_err(|e| {
        if e.codigo == 2 {
            ErrorSolar::nuevo("No se encontró una terminal. Elige una en Ajustes, pestaña Archivos.")
        } else {
            e
        }
    })
}

pub fn detectar_terminal() -> Option<String> {
    let mut buf = [0 as c_char; 512];
    let r = unsafe { ffi::solar_detectar_terminal(buf.as_mut_ptr(), buf.len()) };
    (r == 0).then(|| texto_de_buffer(&buf))
}

pub fn lanzar(argv: &[&OsStr], dir: Option<&Path>) -> Result<(), ErrorSolar> {
    let c: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_bytes().iter().copied().filter(|b| *b != 0).collect::<Vec<_>>()).unwrap_or_default())
        .collect();
    let mut p: Vec<*const c_char> = c.iter().map(|s| s.as_ptr()).collect();
    p.push(ptr::null());
    let d = dir.map(cruta);
    resultado(unsafe { ffi::solar_lanzar(p.as_ptr(), d.as_ref().map_or(ptr::null(), |d| d.as_ptr())) })
}

// ======================================================================
//  Papelera
// ======================================================================

#[derive(Clone, Debug)]
pub struct ElementoPapelera {
    pub nombre: String,
    pub ruta_original: PathBuf,
    pub ruta: PathBuf,
    pub fecha: i64,
    pub tamano: u64,
    pub es_carpeta: bool,
    pub categoria: Categoria,
}

pub fn papelera_listar() -> Vec<ElementoPapelera> {
    let mut p = ffi::SolarPapelera { elementos: ptr::null_mut(), cantidad: 0 };
    unsafe { ffi::solar_papelera_listar(&mut p) };
    let mut v = Vec::with_capacity(p.cantidad);
    for i in 0..p.cantidad {
        let e = unsafe { &*p.elementos.add(i) };
        v.push(ElementoPapelera {
            nombre: unsafe { texto_de(e.nombre) },
            ruta_original: unsafe { ruta_de(e.ruta_original) },
            ruta: unsafe { ruta_de(e.ruta) },
            fecha: e.fecha,
            tamano: e.tamano,
            es_carpeta: e.es_carpeta != 0,
            categoria: Categoria::de(e.categoria),
        });
    }
    unsafe { ffi::solar_papelera_liberar(&mut p) };
    v
}

pub fn papelera_restaurar(ruta: &Path) -> Result<PathBuf, ErrorSolar> {
    let c = cruta(ruta);
    let mut destino = vec![0 as c_char; 4096];
    let mut error = vec![0 as c_char; 1024];
    let r = unsafe {
        ffi::solar_papelera_restaurar(c.as_ptr(), destino.as_mut_ptr(), destino.len(), error.as_mut_ptr(), error.len())
    };
    if r == 0 { Ok(ruta_de_buffer(&destino)) } else { Err(ErrorSolar::de_buffer(r, &error)) }
}

pub fn papelera_borrar(ruta: &Path) -> Result<(), ErrorSolar> {
    let c = cruta(ruta);
    resultado(unsafe { ffi::solar_papelera_borrar(c.as_ptr()) })
}

pub fn papelera_vacia() -> bool {
    unsafe { ffi::solar_papelera_vacia() != 0 }
}

// ======================================================================
//  Dispositivos
// ======================================================================

#[derive(Clone, PartialEq, Debug)]
pub struct Dispositivo {
    pub dispositivo: String,
    pub nombre: String,
    pub punto_montaje: Option<PathBuf>,
    pub sistema_archivos: Option<String>,
    pub tamano: u64,
    pub libre: u64,
    pub extraible: bool,
    pub montado: bool,
    pub solo_lectura: bool,
}

pub fn dispositivos(incluir_internos: bool) -> Vec<Dispositivo> {
    let mut d = ffi::SolarDispositivos { dispositivos: ptr::null_mut(), cantidad: 0 };
    unsafe { ffi::solar_dispositivos_listar(incluir_internos as c_int, &mut d) };
    let mut v = Vec::with_capacity(d.cantidad);
    for i in 0..d.cantidad {
        let e = unsafe { &*d.dispositivos.add(i) };
        v.push(Dispositivo {
            dispositivo: unsafe { texto_de(e.dispositivo) },
            nombre: unsafe { texto_de(e.nombre) },
            punto_montaje: (!e.punto_montaje.is_null()).then(|| unsafe { ruta_de(e.punto_montaje) }),
            sistema_archivos: (!e.sistema_archivos.is_null()).then(|| unsafe { texto_de(e.sistema_archivos) }),
            tamano: e.tamano,
            libre: e.libre,
            extraible: e.extraible != 0,
            montado: e.montado != 0,
            solo_lectura: e.solo_lectura != 0,
        });
    }
    unsafe { ffi::solar_dispositivos_liberar(&mut d) };
    v
}

pub fn montar(dispositivo: &str) -> Result<(), ErrorSolar> {
    let c = ctexto(dispositivo);
    let mut error = vec![0 as c_char; 1024];
    let r = unsafe { ffi::solar_dispositivo_montar(c.as_ptr(), error.as_mut_ptr(), error.len()) };
    if r == 0 { Ok(()) } else { Err(ErrorSolar::de_buffer(r, &error)) }
}

pub fn desmontar(dispositivo: &str, expulsar: bool) -> Result<(), ErrorSolar> {
    let c = ctexto(dispositivo);
    let mut error = vec![0 as c_char; 1024];
    let r = unsafe {
        ffi::solar_dispositivo_desmontar(c.as_ptr(), expulsar as c_int, error.as_mut_ptr(), error.len())
    };
    if r == 0 { Ok(()) } else { Err(ErrorSolar::de_buffer(r, &error)) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Espacio {
    pub total: u64,
    pub libre: u64,
    pub disponible: u64,
}

pub fn espacio(ruta: &Path) -> Option<Espacio> {
    let c = cruta(ruta);
    let mut e = ffi::SolarEspacio::default();
    let r = unsafe { ffi::solar_espacio(c.as_ptr(), &mut e) };
    (r == 0).then_some(Espacio { total: e.total, libre: e.libre, disponible: e.disponible })
}

// ======================================================================
//  Vigilante
// ======================================================================

/// El objeto de C, compartido entre el hilo vigía y quien lo despierta.
/// Se destruye cuando ya nadie lo usa.
struct VigilanteC(*mut ffi::SolarVigilante);

// Solo el hilo vigía llama a observar/esperar; desde otros hilos solo se
// llama a solar_vigilante_despertar, que en C es seguro entre hilos.
unsafe impl Send for VigilanteC {}
unsafe impl Sync for VigilanteC {}

impl Drop for VigilanteC {
    fn drop(&mut self) {
        unsafe { ffi::solar_vigilante_destruir(self.0) };
    }
}

pub struct Vigilante(std::sync::Arc<VigilanteC>);

/// Despierta al hilo vigía desde cualquier otro hilo.
#[derive(Clone)]
pub struct Despertador(std::sync::Arc<VigilanteC>);

pub const CAMBIO_CARPETA: i32 = ffi::SOLAR_CAMBIO_CARPETA;
pub const CAMBIO_MONTAJES: i32 = ffi::SOLAR_CAMBIO_MONTAJES;
pub const CAMBIO_PAPELERA: i32 = ffi::SOLAR_CAMBIO_PAPELERA;
pub const CAMBIO_DISPOSITIVOS: i32 = ffi::SOLAR_CAMBIO_DISPOSITIVOS;

impl Vigilante {
    pub fn nuevo() -> Option<Self> {
        let v = unsafe { ffi::solar_vigilante_crear() };
        (!v.is_null()).then(|| Self(std::sync::Arc::new(VigilanteC(v))))
    }

    pub fn despertador(&self) -> Despertador {
        Despertador(self.0.clone())
    }

    pub fn observar(&mut self, ruta: Option<&Path>) {
        let c = ruta.map(cruta);
        unsafe { ffi::solar_vigilante_observar(self.0.0, c.as_ref().map_or(ptr::null(), |c| c.as_ptr())) };
    }

    /// Duerme hasta que pase algo (o hasta `milisegundos`).
    pub fn esperar(&mut self, milisegundos: i32) -> i32 {
        unsafe { ffi::solar_vigilante_esperar(self.0.0, milisegundos) }
    }
}

impl Despertador {
    pub fn despertar(&self) {
        unsafe { ffi::solar_vigilante_despertar(self.0.0) };
    }
}

// ======================================================================
//  Carpetas personalizadas
// ======================================================================

#[derive(Clone, Default, PartialEq, Debug)]
pub struct EstiloCarpeta {
    pub color: Option<Color32>,
    pub icono: Option<String>,
    pub color_icono: Option<Color32>,
}

impl EstiloCarpeta {
    pub fn vacio(&self) -> bool {
        self.color.is_none() && self.icono.is_none() && self.color_icono.is_none()
    }

    fn a_c(&self) -> ffi::SolarEstiloCarpeta {
        let mut e = ffi::SolarEstiloCarpeta {
            color: self.color.map(color_a).unwrap_or(0),
            color_icono: self.color_icono.map(color_a).unwrap_or(0),
            tiene_color: self.color.is_some() as u8,
            tiene_color_icono: self.color_icono.is_some() as u8,
            _relleno: [0; 2],
            icono: [0; 48],
        };
        if let Some(icono) = &self.icono {
            // Cortamos en un límite de carácter para no partir un emoji
            let mut fin = icono.len().min(47);
            while !icono.is_char_boundary(fin) {
                fin -= 1;
            }
            for (i, b) in icono.as_bytes()[..fin].iter().enumerate() {
                e.icono[i] = *b as c_char;
            }
        }
        e
    }

    fn de_c(e: &ffi::SolarEstiloCarpeta) -> Self {
        let icono = texto_de_buffer(&e.icono);
        Self {
            color: (e.tiene_color != 0).then(|| color_de(e.color)),
            icono: (!icono.is_empty()).then_some(icono),
            color_icono: (e.tiene_color_icono != 0).then(|| color_de(e.color_icono)),
        }
    }
}

pub struct Carpetas(*mut ffi::SolarCarpetas);

// Se usa siempre detrás de un Mutex.
unsafe impl Send for Carpetas {}

impl Carpetas {
    pub fn abrir(archivo: &Path) -> Option<Self> {
        let c = cruta(archivo);
        let db = unsafe { ffi::solar_carpetas_abrir(c.as_ptr()) };
        (!db.is_null()).then_some(Self(db))
    }

    pub fn obtener(&self, ruta: &Path) -> Option<EstiloCarpeta> {
        let c = cruta(ruta);
        let mut e: ffi::SolarEstiloCarpeta = unsafe { std::mem::zeroed() };
        let r = unsafe { ffi::solar_carpetas_obtener(self.0, c.as_ptr(), &mut e) };
        (r == 1).then(|| EstiloCarpeta::de_c(&e))
    }

    pub fn poner(&mut self, ruta: &Path, estilo: &EstiloCarpeta) -> Result<(), ErrorSolar> {
        let c = cruta(ruta);
        let e = estilo.a_c();
        resultado(unsafe { ffi::solar_carpetas_poner(self.0, c.as_ptr(), &e) })
    }

    pub fn quitar(&mut self, ruta: &Path) -> Result<(), ErrorSolar> {
        let c = cruta(ruta);
        resultado(unsafe { ffi::solar_carpetas_quitar(self.0, c.as_ptr()) })
    }

    pub fn olvidar_arbol(&mut self, ruta: &Path) {
        let c = cruta(ruta);
        unsafe { ffi::solar_carpetas_olvidar_arbol(self.0, c.as_ptr()) };
    }

    pub fn renombrar(&mut self, vieja: &Path, nueva: &Path) {
        let (a, b) = (cruta(vieja), cruta(nueva));
        unsafe { ffi::solar_carpetas_renombrar(self.0, a.as_ptr(), b.as_ptr()) };
    }

    pub fn copiar(&mut self, origen: &Path, destino: &Path) {
        let (a, b) = (cruta(origen), cruta(destino));
        unsafe { ffi::solar_carpetas_copiar(self.0, a.as_ptr(), b.as_ptr()) };
    }

    pub fn cantidad(&self) -> usize {
        unsafe { ffi::solar_carpetas_cantidad(self.0) }
    }
}

impl Drop for Carpetas {
    fn drop(&mut self) {
        unsafe { ffi::solar_carpetas_cerrar(self.0) };
    }
}

// ======================================================================
//  Configuración
// ======================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Posicion {
    Arriba,
    Abajo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lado {
    Izquierda,
    Derecha,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vista {
    Cuadricula,
    Lista,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Lugar {
    pub nombre: String,
    pub ruta: String,
    pub icono: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Config {
    pub tema: String,
    pub transparencia: bool,
    pub opacidad: f32,
    pub escala: f32,
    pub fuente: String,
    pub barra_titulo_integrada: bool,
    pub barra_ubicacion: Posicion,
    pub barra_lateral: Lado,
    pub mostrar_barra_lateral: bool,
    pub vista: Vista,
    pub tamano_iconos: f32,
    pub mostrar_ocultos: bool,
    pub carpetas_primero: bool,
    pub orden: Orden,
    pub orden_descendente: bool,
    pub un_clic: bool,
    pub confirmar_papelera: bool,
    pub confirmar_eliminar: bool,
    pub terminal: String,
    pub mostrar_usb: bool,
    pub mostrar_discos_internos: bool,
    pub sonidos: bool,
    pub volumen: f32,
    pub animaciones: bool,
    pub buscar_actualizaciones: bool,
    pub repo_actualizaciones: String,
    pub lugares: Vec<Lugar>,
    pub atajos: Vec<(String, String)>,
}

pub struct CargaConfig {
    pub config: Config,
    pub avisos: Option<String>,
    pub existia: bool,
}

impl Config {
    fn de_c(c: &ffi::SolarConfig) -> Self {
        let mut lugares = Vec::with_capacity(c.n_lugares);
        for i in 0..c.n_lugares {
            let l = unsafe { &*c.lugares.add(i) };
            lugares.push(Lugar {
                nombre: unsafe { texto_de(l.nombre) },
                ruta: unsafe { texto_de(l.ruta) },
                icono: unsafe { texto_de(l.icono) },
            });
        }
        let mut atajos = Vec::with_capacity(c.n_atajos);
        for i in 0..c.n_atajos {
            let a = unsafe { &*c.atajos.add(i) };
            atajos.push((unsafe { texto_de(a.accion) }, unsafe { texto_de(a.teclas) }));
        }
        Self {
            tema: unsafe { texto_de(c.tema) },
            transparencia: c.transparencia != 0,
            opacidad: c.opacidad as f32,
            escala: c.escala as f32,
            fuente: unsafe { texto_de(c.fuente) },
            barra_titulo_integrada: c.barra_titulo_integrada != 0,
            barra_ubicacion: if c.barra_ubicacion == 1 { Posicion::Abajo } else { Posicion::Arriba },
            barra_lateral: if c.barra_lateral == 1 { Lado::Derecha } else { Lado::Izquierda },
            mostrar_barra_lateral: c.mostrar_barra_lateral != 0,
            vista: if c.vista == 1 { Vista::Lista } else { Vista::Cuadricula },
            tamano_iconos: c.tamano_iconos as f32,
            mostrar_ocultos: c.mostrar_ocultos != 0,
            carpetas_primero: c.carpetas_primero != 0,
            orden: Orden::de(c.orden),
            orden_descendente: c.orden_descendente != 0,
            un_clic: c.un_clic != 0,
            confirmar_papelera: c.confirmar_papelera != 0,
            confirmar_eliminar: c.confirmar_eliminar != 0,
            terminal: unsafe { texto_de(c.terminal) },
            mostrar_usb: c.mostrar_usb != 0,
            mostrar_discos_internos: c.mostrar_discos_internos != 0,
            sonidos: c.sonidos != 0,
            volumen: c.volumen as f32,
            animaciones: c.animaciones != 0,
            buscar_actualizaciones: c.buscar_actualizaciones != 0,
            repo_actualizaciones: unsafe { texto_de(c.repo_actualizaciones) },
            lugares,
            atajos,
        }
    }

    pub fn defecto() -> Self {
        let mut c: ffi::SolarConfig = unsafe { std::mem::zeroed() };
        unsafe { ffi::solar_config_defecto(&mut c) };
        let r = Self::de_c(&c);
        unsafe { ffi::solar_config_liberar(&mut c) };
        r
    }

    pub fn cargar(archivo: &Path) -> CargaConfig {
        let mut c: ffi::SolarConfig = unsafe { std::mem::zeroed() };
        unsafe { ffi::solar_config_defecto(&mut c) };
        let ca = cruta(archivo);
        let mut avisos = vec![0 as c_char; 8192];
        let r = unsafe { ffi::solar_config_cargar(ca.as_ptr(), &mut c, avisos.as_mut_ptr(), avisos.len()) };
        let config = Self::de_c(&c);
        unsafe { ffi::solar_config_liberar(&mut c) };
        let texto = texto_de_buffer(&avisos);
        CargaConfig { config, avisos: (!texto.is_empty()).then_some(texto), existia: r != 2 }
    }

    pub fn guardar(&self, archivo: &Path) -> Result<(), ErrorSolar> {
        let tema = ctexto(&self.tema);
        let fuente = ctexto(&self.fuente);
        let terminal = ctexto(&self.terminal);
        let repo = ctexto(&self.repo_actualizaciones);
        let lugares_c: Vec<(CString, CString, CString)> = self
            .lugares
            .iter()
            .map(|l| (ctexto(&l.nombre), ctexto(&l.ruta), ctexto(&l.icono)))
            .collect();
        let mut lugares: Vec<ffi::SolarLugar> = lugares_c
            .iter()
            .map(|(n, r, i)| ffi::SolarLugar {
                nombre: n.as_ptr() as *mut c_char,
                ruta: r.as_ptr() as *mut c_char,
                icono: i.as_ptr() as *mut c_char,
            })
            .collect();
        let atajos_c: Vec<(CString, CString)> = self.atajos.iter().map(|(a, t)| (ctexto(a), ctexto(t))).collect();
        let mut atajos: Vec<ffi::SolarAtajo> = atajos_c
            .iter()
            .map(|(a, t)| ffi::SolarAtajo { accion: a.as_ptr() as *mut c_char, teclas: t.as_ptr() as *mut c_char })
            .collect();
        let c = ffi::SolarConfig {
            tema: tema.as_ptr() as *mut c_char,
            transparencia: self.transparencia as i32,
            opacidad: self.opacidad as f64,
            escala: self.escala as f64,
            fuente: fuente.as_ptr() as *mut c_char,
            barra_titulo_integrada: self.barra_titulo_integrada as i32,
            barra_ubicacion: (self.barra_ubicacion == Posicion::Abajo) as i32,
            barra_lateral: (self.barra_lateral == Lado::Derecha) as i32,
            mostrar_barra_lateral: self.mostrar_barra_lateral as i32,
            vista: (self.vista == Vista::Lista) as i32,
            tamano_iconos: self.tamano_iconos.round() as i32,
            mostrar_ocultos: self.mostrar_ocultos as i32,
            carpetas_primero: self.carpetas_primero as i32,
            orden: self.orden as i32,
            orden_descendente: self.orden_descendente as i32,
            un_clic: self.un_clic as i32,
            confirmar_papelera: self.confirmar_papelera as i32,
            confirmar_eliminar: self.confirmar_eliminar as i32,
            terminal: terminal.as_ptr() as *mut c_char,
            mostrar_usb: self.mostrar_usb as i32,
            mostrar_discos_internos: self.mostrar_discos_internos as i32,
            sonidos: self.sonidos as i32,
            volumen: self.volumen as f64,
            animaciones: self.animaciones as i32,
            buscar_actualizaciones: self.buscar_actualizaciones as i32,
            repo_actualizaciones: repo.as_ptr() as *mut c_char,
            lugares: lugares.as_mut_ptr(),
            n_lugares: lugares.len(),
            atajos: atajos.as_mut_ptr(),
            n_atajos: atajos.len(),
        };
        let a = cruta(archivo);
        resultado(unsafe { ffi::solar_config_guardar(a.as_ptr(), &c) })
    }
}

pub fn atajos_defecto() -> Vec<(String, String)> {
    let mut tabla: *const ffi::SolarAtajo = ptr::null();
    let n = unsafe { ffi::solar_atajos_defecto(&mut tabla) };
    (0..n)
        .map(|i| {
            let a = unsafe { &*tabla.add(i) };
            (unsafe { texto_de(a.accion) }, unsafe { texto_de(a.teclas) })
        })
        .collect()
}

// ======================================================================
//  Temas
// ======================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorTema {
    Fondo = 0,
    Superficie,
    Elevado,
    Borde,
    Texto,
    TextoTenue,
    Acento,
    TextoAcento,
    Seleccion,
    Peligro,
    Exito,
    Aviso,
    Carpeta,
}

impl ColorTema {
    pub const TODOS: [ColorTema; 13] = [
        ColorTema::Fondo,
        ColorTema::Superficie,
        ColorTema::Elevado,
        ColorTema::Borde,
        ColorTema::Texto,
        ColorTema::TextoTenue,
        ColorTema::Acento,
        ColorTema::TextoAcento,
        ColorTema::Seleccion,
        ColorTema::Peligro,
        ColorTema::Exito,
        ColorTema::Aviso,
        ColorTema::Carpeta,
    ];

    pub fn nombre(self) -> &'static str {
        match self {
            Self::Fondo => "Fondo",
            Self::Superficie => "Barras",
            Self::Elevado => "Menús y botones",
            Self::Borde => "Bordes",
            Self::Texto => "Texto",
            Self::TextoTenue => "Texto secundario",
            Self::Acento => "Acento",
            Self::TextoAcento => "Texto sobre acento",
            Self::Seleccion => "Selección",
            Self::Peligro => "Peligro",
            Self::Exito => "Éxito",
            Self::Aviso => "Aviso",
            Self::Carpeta => "Carpetas",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tema {
    pub id: String,
    pub nombre: String,
    pub ruta: Option<PathBuf>,
    pub oscuro: bool,
    pub integrado: bool,
    pub colores: [Color32; 13],
}

impl Tema {
    pub fn c(&self, c: ColorTema) -> Color32 {
        self.colores[c as usize]
    }
}

pub fn cargar_temas(dir: Option<&Path>) -> (Vec<Tema>, Option<String>) {
    let d = dir.map(cruta);
    let mut t = ffi::SolarTemas { temas: ptr::null_mut(), cantidad: 0 };
    let mut avisos = vec![0 as c_char; 8192];
    unsafe {
        ffi::solar_temas_cargar(
            d.as_ref().map_or(ptr::null(), |d| d.as_ptr()),
            &mut t,
            avisos.as_mut_ptr(),
            avisos.len(),
        )
    };
    let mut v = Vec::with_capacity(t.cantidad);
    for i in 0..t.cantidad {
        let e = unsafe { &*t.temas.add(i) };
        let mut colores = [Color32::BLACK; 13];
        for (j, c) in colores.iter_mut().enumerate() {
            *c = color_de(e.colores[j]);
        }
        v.push(Tema {
            id: unsafe { texto_de(e.id) },
            nombre: unsafe { texto_de(e.nombre) },
            ruta: (!e.ruta.is_null()).then(|| unsafe { ruta_de(e.ruta) }),
            oscuro: e.oscuro != 0,
            integrado: e.integrado != 0,
            colores,
        });
    }
    unsafe { ffi::solar_temas_liberar(&mut t) };
    let texto = texto_de_buffer(&avisos);
    (v, (!texto.is_empty()).then_some(texto))
}

pub fn guardar_tema(archivo: &Path, tema: &Tema) -> Result<(), ErrorSolar> {
    let id = ctexto(&tema.id);
    let nombre = ctexto(&tema.nombre);
    let mut colores = [0u32; 13];
    for (i, c) in tema.colores.iter().enumerate() {
        colores[i] = color_a(*c);
    }
    let t = ffi::SolarTema {
        id: id.as_ptr() as *mut c_char,
        nombre: nombre.as_ptr() as *mut c_char,
        ruta: ptr::null_mut(),
        oscuro: tema.oscuro as i32,
        integrado: 0,
        colores,
    };
    let a = cruta(archivo);
    resultado(unsafe { ffi::solar_tema_guardar(a.as_ptr(), &t) })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn version_y_errores() {
        assert_eq!(version(), "0.1.1-alpha");
        assert_eq!(texto_error(13), "Permiso denegado");
    }

    #[test]
    fn listar_y_estilos() {
        let dir = std::env::temp_dir().join(format!("solar-rust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Juegos")).unwrap();
        std::fs::write(dir.join("foto10.png"), b"x").unwrap();
        std::fs::write(dir.join("foto2.png"), b"xy").unwrap();
        let v = listar(&dir, Orden::Nombre, false, true).unwrap();
        let nombres: Vec<_> = v.iter().map(|e| e.nombre.as_str()).collect();
        assert_eq!(nombres, ["Juegos", "foto2.png", "foto10.png"]);
        assert_eq!(&*v[1].descripcion, "Imagen PNG");
        assert!(v[0].es_carpeta());

        let mut db = Carpetas::abrir(&dir.join("carpetas.tsv")).unwrap();
        let estilo = EstiloCarpeta {
            color: Some(Color32::from_rgb(0x15, 0x80, 0x3d)),
            icono: Some("mando".into()),
            color_icono: Some(Color32::from_rgb(0x14, 0x53, 0x2d)),
        };
        db.poner(&dir.join("Juegos"), &estilo).unwrap();
        assert_eq!(db.obtener(&dir.join("Juegos")), Some(estilo));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_ida_y_vuelta() {
        let dir = std::env::temp_dir().join(format!("solar-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut c = Config::defecto();
        c.tema = "nebulosa".into();
        c.barra_ubicacion = Posicion::Abajo;
        c.lugares.push(Lugar { nombre: "Juegos \"retro\"".into(), ruta: "~/Juegos".into(), icono: "mando".into() });
        let archivo = dir.join("config.lua");
        c.guardar(&archivo).unwrap();
        let leida = Config::cargar(&archivo);
        assert!(leida.avisos.is_none(), "{:?}", leida.avisos);
        assert_eq!(leida.config, c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn los_ejemplos_del_repo_se_entienden() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR"));
        let carga = Config::cargar(&raiz.join("config/config.lua"));
        assert!(carga.existia);
        assert!(carga.avisos.is_none(), "{:?}", carga.avisos);
        assert_eq!(carga.config.lugares.len(), 5);
        assert_eq!(carga.config.lugares[4].nombre, "Música");
        let eliminar = carga.config.atajos.iter().find(|(a, _)| a == "eliminar").unwrap();
        assert_eq!(eliminar.1, "");

        let (temas, avisos) = cargar_temas(Some(&raiz.join("config/temas")));
        assert!(avisos.is_none(), "{avisos:?}");
        let ejemplo = temas.iter().find(|t| t.id == "ejemplo").expect("tema de ejemplo");
        assert_eq!(ejemplo.nombre, "Bosque");
        assert_eq!(ejemplo.c(ColorTema::Acento), Color32::from_rgb(0x22, 0xC5, 0x5E));
        assert_eq!(ejemplo.c(ColorTema::Seleccion), Color32::from_rgb(0x22, 0xC5, 0x5E));
        assert_eq!(ejemplo.c(ColorTema::Carpeta), Color32::from_rgb(0x15, 0x80, 0x3D));
    }

    #[test]
    fn temas_integrados() {
        let (temas, avisos) = cargar_temas(None);
        assert!(avisos.is_none());
        assert_eq!(temas.len(), 9);
        assert_eq!(temas[0].c(ColorTema::Acento), Color32::from_rgb(0xF9, 0x73, 0x16));
    }
}

// ======================================================================
//  Temas renombrados, sonido y programas
// ======================================================================

/// Id vigente de un tema con nombre viejo ("solar" → "sol"…).
pub fn tema_id_actual(id: &str) -> String {
    let c = ctexto(id);
    unsafe { texto_de(ffi::solar_tema_id_actual(c.as_ptr())) }
}

pub fn sonido_cargar(id: i32, mp3: &[u8]) -> Result<(), ErrorSolar> {
    resultado(unsafe { ffi::solar_sonido_cargar(id, mp3.as_ptr(), mp3.len()) })
}

pub fn sonido_reproducir(id: i32, volumen: f32) {
    unsafe { ffi::solar_sonido_reproducir(id, volumen) };
}

/// "PulseAudio / PipeWire", "ALSA" o "" si no hay por dónde sonar.
pub fn sonido_salida() -> String {
    unsafe { texto_de(ffi::solar_sonido_salida()) }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Afinidad {
    Otro = 0,
    Padre = 1,
    Exacta = 2,
    Predeterminado = 3,
}

/// Un programa instalado (de su lanzador .desktop).
#[derive(Clone, Debug)]
pub struct Programa {
    pub id: String,
    pub archivo: String,
    pub nombre: String,
    pub descripcion: String,
    pub icono: String,
    pub categorias: String,
    pub terminal: bool,
    pub afinidad: Afinidad,
}

pub struct ListaProgramas {
    pub programas: Vec<Programa>,
    pub mime: String,
}

pub fn programas_para(rutas: &[PathBuf], incluir_todos: bool) -> Result<ListaProgramas, ErrorSolar> {
    let (_c, p) = arreglo(rutas);
    let mut s: ffi::SolarProgramas = unsafe { std::mem::zeroed() };
    resultado(unsafe { ffi::solar_programas_para(p.as_ptr(), p.len(), incluir_todos as i32, &mut s) })?;
    let mut v = Vec::with_capacity(s.cantidad);
    for i in 0..s.cantidad {
        let x = unsafe { &*s.programas.add(i) };
        v.push(Programa {
            id: unsafe { texto_de(x.id) },
            archivo: unsafe { texto_de(x.archivo) },
            nombre: unsafe { texto_de(x.nombre) },
            descripcion: unsafe { texto_de(x.descripcion) },
            icono: unsafe { texto_de(x.icono) },
            categorias: unsafe { texto_de(x.categorias) },
            terminal: x.terminal != 0,
            afinidad: match x.afinidad {
                3 => Afinidad::Predeterminado,
                2 => Afinidad::Exacta,
                1 => Afinidad::Padre,
                _ => Afinidad::Otro,
            },
        });
    }
    let mime = unsafe { texto_de(s.mime) };
    unsafe { ffi::solar_programas_liberar(&mut s) };
    Ok(ListaProgramas { programas: v, mime })
}

pub fn programa_lanzar(p: &Programa, rutas: &[PathBuf], terminal: &str) -> Result<(), ErrorSolar> {
    let id = ctexto(&p.id);
    let archivo = ctexto(&p.archivo);
    let c = ffi::SolarPrograma {
        id: id.as_ptr() as *mut c_char,
        archivo: archivo.as_ptr() as *mut c_char,
        nombre: ptr::null_mut(),
        descripcion: ptr::null_mut(),
        icono: ptr::null_mut(),
        categorias: ptr::null_mut(),
        terminal: p.terminal as i32,
        afinidad: p.afinidad as i32,
    };
    let (_r, pr) = arreglo(rutas);
    let t = ctexto(terminal);
    resultado(unsafe { ffi::solar_programa_lanzar(&c, pr.as_ptr(), pr.len(), t.as_ptr()) }).map_err(|e| {
        if e.codigo == 2 && p.terminal {
            ErrorSolar { codigo: e.codigo, mensaje: format!("«{}» se abre en una terminal y no encontré ninguna. Elige una en Ajustes → Archivos.", p.nombre) }
        } else {
            ErrorSolar { codigo: e.codigo, mensaje: format!("No se pudo abrir con «{}»: {}", p.nombre, e.mensaje) }
        }
    })
}

pub fn programa_predeterminar(id: &str, mime: &str) -> Result<(), ErrorSolar> {
    let i = ctexto(id);
    let m = ctexto(mime);
    resultado(unsafe { ffi::solar_programa_predeterminar(i.as_ptr(), m.as_ptr()) })
}
