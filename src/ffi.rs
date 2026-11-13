//! Enlaces crudos con el núcleo en C (`nucleo/include/solar.h`).
//!
//! Este archivo es un espejo exacto de solar.h: si cambias una estructura
//! allá, cámbiala aquí también. El resto de la interfaz usa `nucleo.rs`,
//! que envuelve todo esto con tipos seguros de Rust.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};

pub type ProgresoFn =
    Option<unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, *const c_char) -> c_int>;
pub type ResultadoFn = Option<unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char)>;

pub const SOLAR_TIPO_ARCHIVO: u8 = 0;
pub const SOLAR_TIPO_CARPETA: u8 = 1;
pub const SOLAR_TIPO_ENLACE_ROTO: u8 = 2;
pub const SOLAR_TIPO_ESPECIAL: u8 = 3;

pub const SOLAR_CAMBIO_CARPETA: c_int = 1;
pub const SOLAR_CAMBIO_MONTAJES: c_int = 2;
pub const SOLAR_CAMBIO_PAPELERA: c_int = 4;
pub const SOLAR_CAMBIO_DISPOSITIVOS: c_int = 8;

pub const SOLAR_COLORES_TOTAL: usize = 13;

#[repr(C)]
pub struct SolarEntrada {
    pub nombre: *mut c_char,
    pub tamano: u64,
    pub modificado: i64,
    pub modo: u32,
    pub tipo: u8,
    pub categoria: u8,
    pub es_enlace: u8,
    pub es_oculto: u8,
}

#[repr(C)]
pub struct SolarListado {
    pub entradas: *mut SolarEntrada,
    pub cantidad: usize,
}

#[repr(C)]
pub struct SolarInfo {
    pub tamano: u64,
    pub modificado: i64,
    pub accedido: i64,
    pub creado: i64,
    pub modo: u32,
    pub uid: u32,
    pub gid: u32,
    pub tipo: u8,
    pub categoria: u8,
    pub es_enlace: u8,
    pub _relleno: u8,
    pub propietario: [c_char; 64],
    pub grupo: [c_char; 64],
    pub destino_enlace: [c_char; 1024],
}

#[repr(C)]
pub struct SolarOpciones {
    pub conflicto: i32,
    pub progreso: ProgresoFn,
    pub resultado: ResultadoFn,
    pub datos: *mut c_void,
    pub error: *mut c_char,
    pub error_tam: usize,
}

#[repr(C)]
pub struct SolarElementoPapelera {
    pub nombre: *mut c_char,
    pub ruta_original: *mut c_char,
    pub ruta: *mut c_char,
    pub fecha: i64,
    pub tamano: u64,
    pub es_carpeta: u8,
    pub categoria: u8,
    pub _relleno: [u8; 6],
}

#[repr(C)]
pub struct SolarPapelera {
    pub elementos: *mut SolarElementoPapelera,
    pub cantidad: usize,
}

#[repr(C)]
pub struct SolarDispositivo {
    pub dispositivo: *mut c_char,
    pub nombre: *mut c_char,
    pub punto_montaje: *mut c_char,
    pub sistema_archivos: *mut c_char,
    pub tamano: u64,
    pub libre: u64,
    pub extraible: u8,
    pub montado: u8,
    pub solo_lectura: u8,
    pub _relleno: [u8; 5],
}

#[repr(C)]
pub struct SolarDispositivos {
    pub dispositivos: *mut SolarDispositivo,
    pub cantidad: usize,
}

#[repr(C)]
#[derive(Default)]
pub struct SolarEspacio {
    pub total: u64,
    pub libre: u64,
    pub disponible: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolarEstiloCarpeta {
    pub color: u32,
    pub color_icono: u32,
    pub tiene_color: u8,
    pub tiene_color_icono: u8,
    pub _relleno: [u8; 2],
    pub icono: [c_char; 48],
}

#[repr(C)]
pub struct SolarLugar {
    pub nombre: *mut c_char,
    pub ruta: *mut c_char,
    pub icono: *mut c_char,
}

#[repr(C)]
pub struct SolarAtajo {
    pub accion: *mut c_char,
    pub teclas: *mut c_char,
}

#[repr(C)]
pub struct SolarConfig {
    pub tema: *mut c_char,
    pub transparencia: i32,
    pub opacidad: f64,
    pub escala: f64,
    pub fuente: *mut c_char,
    pub barra_titulo_integrada: i32,
    pub barra_ubicacion: i32,
    pub barra_lateral: i32,
    pub mostrar_barra_lateral: i32,
    pub vista: i32,
    pub archivos: SolarAjustesVista,
    pub un_clic: i32,
    pub confirmar_papelera: i32,
    pub confirmar_eliminar: i32,
    pub terminal: *mut c_char,
    pub restaurar_sesion: i32,
    pub mostrar_usb: i32,
    pub mostrar_discos_internos: i32,
    pub sonidos: i32,
    pub volumen: f64,
    pub animaciones: i32,
    pub buscar_actualizaciones: i32,
    pub repo_actualizaciones: *mut c_char,
    pub lugares: *mut SolarLugar,
    pub n_lugares: usize,
    pub atajos: *mut SolarAtajo,
    pub n_atajos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolarAjustesVista {
    pub tamano_iconos: i32,
    pub mostrar_ocultos: i32,
    pub carpetas_primero: i32,
    pub orden: i32,
    pub orden_descendente: i32,
    pub orbita_girar: i32,
    pub orbita_velocidad: f64,
    pub orbita_orden: i32,
    pub orbita_electricidad: i32,
}

#[repr(C)]
pub struct SolarPestana {
    pub ruta: *mut c_char,
    pub papelera: i32,
    pub vista: i32,
}

#[repr(C)]
pub struct SolarEspacioTrabajo {
    pub id: *mut c_char,
    pub nombre: *mut c_char,
    pub icono: *mut c_char,
    pub color: *mut c_char,
    pub tema: *mut c_char,
    pub vista: i32,
    pub ajustes_propios: i32,
    pub ajustes: SolarAjustesVista,
    pub pestanas: *mut SolarPestana,
    pub n_pestanas: usize,
    pub activa: i32,
}

#[repr(C)]
pub struct SolarEspacios {
    pub activo: *mut c_char,
    pub espacios: *mut SolarEspacioTrabajo,
    pub n_espacios: usize,
}

#[repr(C)]
pub struct SolarPrograma {
    pub id: *mut c_char,
    pub archivo: *mut c_char,
    pub nombre: *mut c_char,
    pub descripcion: *mut c_char,
    pub icono: *mut c_char,
    pub categorias: *mut c_char,
    pub terminal: i32,
    pub afinidad: i32,
}

#[repr(C)]
pub struct SolarProgramas {
    pub programas: *mut SolarPrograma,
    pub cantidad: usize,
    pub mime: *mut c_char,
}

#[repr(C)]
pub struct SolarTema {
    pub id: *mut c_char,
    pub nombre: *mut c_char,
    pub ruta: *mut c_char,
    pub oscuro: i32,
    pub integrado: i32,
    pub colores: [u32; SOLAR_COLORES_TOTAL],
}

#[repr(C)]
pub struct SolarTemas {
    pub temas: *mut SolarTema,
    pub cantidad: usize,
}

#[repr(C)]
pub struct SolarVigilante {
    _privado: [u8; 0],
}

#[repr(C)]
pub struct SolarCarpetas {
    _privado: [u8; 0],
}

unsafe extern "C" {
    pub fn solar_version() -> *const c_char;
    pub fn solar_error_texto(codigo: c_int) -> *const c_char;
    pub fn solar_expandir_ruta(ruta: *const c_char, salida: *mut c_char, tam: usize) -> c_int;
    pub fn solar_dir_config(salida: *mut c_char, tam: usize) -> c_int;
    pub fn solar_dir_datos(salida: *mut c_char, tam: usize) -> c_int;
    pub fn solar_dir_hogar(salida: *mut c_char, tam: usize) -> c_int;
    pub fn solar_fecha_local(instante: i64, campos: *mut i32);

    pub fn solar_listar(ruta: *const c_char, salida: *mut SolarListado) -> c_int;
    pub fn solar_ordenar(l: *mut SolarListado, orden: c_int, descendente: c_int, carpetas_primero: c_int);
    pub fn solar_listado_liberar(l: *mut SolarListado);
    pub fn solar_comparar_natural(a: *const c_char, b: *const c_char) -> c_int;
    pub fn solar_categoria(nombre: *const c_char, modo: u32, tipo: c_int) -> c_int;
    pub fn solar_describir_tipo(
        nombre: *const c_char,
        categoria: c_int,
        tipo: c_int,
        es_enlace: c_int,
        salida: *mut c_char,
        tam: usize,
    );

    pub fn solar_info(ruta: *const c_char, info: *mut SolarInfo) -> c_int;
    pub fn solar_cambiar_permisos(ruta: *const c_char, modo: u32) -> c_int;
    pub fn solar_tamano_total(
        ruta: *const c_char,
        bytes: *mut u64,
        archivos: *mut u64,
        carpetas: *mut u64,
        progreso: ProgresoFn,
        datos: *mut c_void,
    ) -> c_int;

    pub fn solar_copiar(
        origenes: *const *const c_char,
        cantidad: usize,
        destino: *const c_char,
        opciones: *const SolarOpciones,
    ) -> c_int;
    pub fn solar_mover(
        origenes: *const *const c_char,
        cantidad: usize,
        destino: *const c_char,
        opciones: *const SolarOpciones,
    ) -> c_int;
    pub fn solar_eliminar(rutas: *const *const c_char, cantidad: usize, opciones: *const SolarOpciones) -> c_int;
    pub fn solar_renombrar(ruta: *const c_char, nombre_nuevo: *const c_char) -> c_int;
    pub fn solar_crear_carpeta(dir: *const c_char, nombre: *const c_char) -> c_int;
    pub fn solar_crear_archivo(dir: *const c_char, nombre: *const c_char) -> c_int;
    pub fn solar_validar_nombre(nombre: *const c_char) -> c_int;
    pub fn solar_nombre_libre(
        dir: *const c_char,
        nombre: *const c_char,
        es_copia: c_int,
        salida: *mut c_char,
        tam: usize,
    ) -> c_int;

    pub fn solar_abrir(ruta: *const c_char) -> c_int;
    pub fn solar_abrir_con(comando: *const c_char, rutas: *const *const c_char, cantidad: usize) -> c_int;
    pub fn solar_abrir_terminal(terminal: *const c_char, dir: *const c_char) -> c_int;
    pub fn solar_detectar_terminal(salida: *mut c_char, tam: usize) -> c_int;
    pub fn solar_lanzar(argv: *const *const c_char, dir_trabajo: *const c_char) -> c_int;

    pub fn solar_papelera_enviar(rutas: *const *const c_char, cantidad: usize, opciones: *const SolarOpciones) -> c_int;
    pub fn solar_papelera_listar(salida: *mut SolarPapelera) -> c_int;
    pub fn solar_papelera_liberar(p: *mut SolarPapelera);
    pub fn solar_papelera_restaurar(
        ruta: *const c_char,
        restaurado: *mut c_char,
        restaurado_tam: usize,
        error: *mut c_char,
        error_tam: usize,
    ) -> c_int;
    pub fn solar_papelera_borrar(ruta: *const c_char) -> c_int;
    pub fn solar_papelera_vaciar(opciones: *const SolarOpciones) -> c_int;
    pub fn solar_papelera_vacia() -> c_int;
    pub fn solar_papelera_dir(salida: *mut c_char, tam: usize) -> c_int;

    pub fn solar_dispositivos_listar(incluir_internos: c_int, salida: *mut SolarDispositivos) -> c_int;
    pub fn solar_dispositivos_liberar(d: *mut SolarDispositivos);
    pub fn solar_dispositivo_montar(dispositivo: *const c_char, error: *mut c_char, tam: usize) -> c_int;
    pub fn solar_dispositivo_desmontar(
        dispositivo: *const c_char,
        expulsar: c_int,
        error: *mut c_char,
        tam: usize,
    ) -> c_int;
    pub fn solar_espacio(ruta: *const c_char, espacio: *mut SolarEspacio) -> c_int;

    pub fn solar_vigilante_crear() -> *mut SolarVigilante;
    pub fn solar_vigilante_observar(v: *mut SolarVigilante, ruta: *const c_char) -> c_int;
    pub fn solar_vigilante_esperar(v: *mut SolarVigilante, milisegundos: c_int) -> c_int;
    pub fn solar_vigilante_despertar(v: *mut SolarVigilante);
    pub fn solar_vigilante_destruir(v: *mut SolarVigilante);

    pub fn solar_carpetas_abrir(archivo: *const c_char) -> *mut SolarCarpetas;
    pub fn solar_carpetas_obtener(
        db: *const SolarCarpetas,
        ruta: *const c_char,
        estilo: *mut SolarEstiloCarpeta,
    ) -> c_int;
    pub fn solar_carpetas_poner(db: *mut SolarCarpetas, ruta: *const c_char, estilo: *const SolarEstiloCarpeta) -> c_int;
    pub fn solar_carpetas_quitar(db: *mut SolarCarpetas, ruta: *const c_char) -> c_int;
    pub fn solar_carpetas_olvidar_arbol(db: *mut SolarCarpetas, ruta: *const c_char) -> c_int;
    pub fn solar_carpetas_renombrar(db: *mut SolarCarpetas, vieja: *const c_char, nueva: *const c_char) -> c_int;
    pub fn solar_carpetas_copiar(db: *mut SolarCarpetas, origen: *const c_char, destino: *const c_char) -> c_int;
    pub fn solar_carpetas_cantidad(db: *const SolarCarpetas) -> usize;
    pub fn solar_carpetas_cerrar(db: *mut SolarCarpetas);

    pub fn solar_config_defecto(c: *mut SolarConfig);
    pub fn solar_config_cargar(archivo: *const c_char, c: *mut SolarConfig, avisos: *mut c_char, tam: usize) -> c_int;
    pub fn solar_config_guardar(archivo: *const c_char, c: *const SolarConfig) -> c_int;
    pub fn solar_config_liberar(c: *mut SolarConfig);
    pub fn solar_atajos_defecto(tabla: *mut *const SolarAtajo) -> usize;
    pub fn solar_ajustes_vista_defecto(a: *mut SolarAjustesVista);

    // Espacios de trabajo
    pub fn solar_espacios_cargar(
        archivo: *const c_char,
        base: *const SolarAjustesVista,
        e: *mut SolarEspacios,
        avisos: *mut c_char,
        tam: usize,
    ) -> c_int;
    pub fn solar_espacios_guardar(archivo: *const c_char, e: *const SolarEspacios) -> c_int;
    pub fn solar_espacios_liberar(e: *mut SolarEspacios);

    pub fn solar_temas_cargar(dir_usuario: *const c_char, salida: *mut SolarTemas, avisos: *mut c_char, tam: usize) -> c_int;
    pub fn solar_temas_liberar(t: *mut SolarTemas);
    pub fn solar_tema_guardar(archivo: *const c_char, tema: *const SolarTema) -> c_int;
    pub fn solar_color_clave(indice: c_int) -> *const c_char;

    // Temas renombrados
    pub fn solar_tema_id_actual(id: *const c_char) -> *const c_char;

    // Sonido
    pub fn solar_sonido_cargar(id: i32, mp3: *const u8, tam: usize) -> c_int;
    pub fn solar_sonido_reproducir(id: i32, volumen: f32) -> c_int;
    pub fn solar_sonido_salida() -> *const c_char;

    // Programas
    pub fn solar_mime_de(ruta: *const c_char, mime: *mut c_char, tam: usize) -> c_int;
    pub fn solar_programas_para(rutas: *const *const c_char, cantidad: usize, incluir_todos: i32, salida: *mut SolarProgramas) -> c_int;
    pub fn solar_programas_liberar(p: *mut SolarProgramas);
    pub fn solar_programa_lanzar(p: *const SolarPrograma, rutas: *const *const c_char, cantidad: usize, terminal: *const c_char) -> c_int;
    pub fn solar_programa_predeterminar(id: *const c_char, mime: *const c_char) -> c_int;
}
