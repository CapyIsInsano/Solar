/*
 * ☀ Solar — gestor de archivos
 * API pública del núcleo (C11)
 *
 * El núcleo hace todo el trabajo "de sistema": listar carpetas, copiar,
 * mover, papelera, memorias USB, vigilar cambios, recordar el color y el
 * logotipo de cada carpeta, y leer/escribir la configuración en Lua.
 * La interfaz (Rust) solo dibuja y llama a estas funciones.
 *
 * Convenciones:
 *   - Las funciones que pueden fallar devuelven 0 si todo salió bien, o un
 *     código errno (> 0). solar_error_texto() da el mensaje en español.
 *   - Las rutas son bytes terminados en '\0' (normalmente UTF-8).
 *   - Lo que el núcleo reserva lo libera el núcleo (funciones *_liberar).
 *
 * Licencia: GPL-3.0-or-later
 */
#ifndef SOLAR_H
#define SOLAR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SOLAR_VERSION "0.1.0-alpha"

/* ===================================================================== */
/*  General                                                              */
/* ===================================================================== */

const char *solar_version(void);

/* Mensaje en español para un código errno. Nunca devuelve NULL. */
const char *solar_error_texto(int codigo);

/* "~" y "~/algo" -> ruta absoluta. Devuelve 0 o ENAMETOOLONG. */
int solar_expandir_ruta(const char *ruta, char *salida, size_t tam);

/* $XDG_CONFIG_HOME/solar y $XDG_DATA_HOME/solar (los crea si no existen). */
int solar_dir_config(char *salida, size_t tam);
int solar_dir_datos(char *salida, size_t tam);

/* Carpeta personal del usuario. */
int solar_dir_hogar(char *salida, size_t tam);

/* Fecha local de un instante: año, mes (1-12), día, hora, minuto,
 * segundo y día de la semana (0 = domingo). */
void solar_fecha_local(int64_t instante, int32_t campos[7]);

/* Progreso de operaciones largas. Devuelve distinto de 0 para cancelar. */
typedef int (*SolarProgresoFn)(void *datos,
                               uint64_t bytes_hechos, uint64_t bytes_total,
                               uint64_t elementos_hechos, uint64_t elementos_total,
                               const char *actual);

/* Se llama una vez por cada elemento de primer nivel ya terminado, con su
 * ruta de origen y la ruta final (que puede cambiar si hubo conflicto). */
typedef void (*SolarResultadoFn)(void *datos, const char *origen, const char *destino);

/* ===================================================================== */
/*  Listado de carpetas                                                  */
/* ===================================================================== */

enum {
    SOLAR_TIPO_ARCHIVO = 0,
    SOLAR_TIPO_CARPETA = 1,
    SOLAR_TIPO_ENLACE_ROTO = 2,
    SOLAR_TIPO_ESPECIAL = 3     /* socket, fifo, dispositivo */
};

/* Categorías: deciden el ícono y el orden "por tipo". */
enum {
    SOLAR_CAT_CARPETA = 0,
    SOLAR_CAT_GENERICO,
    SOLAR_CAT_TEXTO,
    SOLAR_CAT_CODIGO,
    SOLAR_CAT_IMAGEN,
    SOLAR_CAT_AUDIO,
    SOLAR_CAT_VIDEO,
    SOLAR_CAT_COMPRIMIDO,
    SOLAR_CAT_PDF,
    SOLAR_CAT_DOCUMENTO,
    SOLAR_CAT_HOJA_CALCULO,
    SOLAR_CAT_PRESENTACION,
    SOLAR_CAT_EJECUTABLE,
    SOLAR_CAT_FUENTE,
    SOLAR_CAT_DISCO,
    SOLAR_CAT_PAQUETE,
    SOLAR_CAT_TOTAL
};

typedef struct {
    char    *nombre;
    uint64_t tamano;        /* bytes (0 en carpetas) */
    int64_t  modificado;    /* segundos desde 1970 */
    uint32_t modo;          /* st_mode */
    uint8_t  tipo;          /* SOLAR_TIPO_* (siguiendo enlaces) */
    uint8_t  categoria;     /* SOLAR_CAT_* */
    uint8_t  es_enlace;     /* es un enlace simbólico */
    uint8_t  es_oculto;     /* empieza con '.', termina en '~' o sale en .hidden */
} SolarEntrada;

typedef struct {
    SolarEntrada *entradas;
    size_t        cantidad;
} SolarListado;

enum {
    SOLAR_ORDEN_NOMBRE = 0,
    SOLAR_ORDEN_TAMANO = 1,
    SOLAR_ORDEN_FECHA = 2,
    SOLAR_ORDEN_TIPO = 3
};

int  solar_listar(const char *ruta, SolarListado *salida);
void solar_ordenar(SolarListado *listado, int orden, int descendente, int carpetas_primero);
void solar_listado_liberar(SolarListado *listado);

/* Comparación "natural": foto2 < foto10, sin distinguir mayúsculas ni
 * acentos (á = a), y con la ñ después de la n. */
int  solar_comparar_natural(const char *a, const char *b);
int  solar_categoria(const char *nombre, uint32_t modo, int tipo);
/* Descripción legible ("Imagen PNG", "Carpeta", "Código fuente Rust"…). */
void solar_describir_tipo(const char *nombre, int categoria, int tipo, int es_enlace,
                          char *salida, size_t tam);

/* ===================================================================== */
/*  Información (Propiedades)                                            */
/* ===================================================================== */

typedef struct {
    uint64_t tamano;
    int64_t  modificado;
    int64_t  accedido;
    int64_t  creado;            /* 0 si el sistema de archivos no lo guarda */
    uint32_t modo;
    uint32_t uid;
    uint32_t gid;
    uint8_t  tipo;
    uint8_t  categoria;
    uint8_t  es_enlace;
    uint8_t  _relleno;
    char     propietario[64];
    char     grupo[64];
    char     destino_enlace[1024];
} SolarInfo;

int solar_info(const char *ruta, SolarInfo *info);
int solar_cambiar_permisos(const char *ruta, uint32_t modo);

/* Tamaño total de una carpeta (no cruza a otros dispositivos).
 * El progreso informa bytes contados y elementos vistos; se puede cancelar. */
int solar_tamano_total(const char *ruta, uint64_t *bytes, uint64_t *archivos,
                       uint64_t *carpetas, SolarProgresoFn progreso, void *datos);

/* ===================================================================== */
/*  Operaciones con archivos                                             */
/* ===================================================================== */

enum {
    SOLAR_CONFLICTO_RENOMBRAR = 0,  /* conservar ambos: "foto (2).png"      */
    SOLAR_CONFLICTO_REEMPLAZAR = 1, /* reemplazar (las carpetas se combinan) */
    SOLAR_CONFLICTO_OMITIR = 2      /* no tocar lo que ya existe            */
};

typedef struct {
    int32_t          conflicto;     /* SOLAR_CONFLICTO_* */
    SolarProgresoFn  progreso;      /* puede ser NULL */
    SolarResultadoFn resultado;     /* puede ser NULL */
    void            *datos;         /* se pasa a los dos callbacks */
    char            *error;         /* mensaje detallado (puede ser NULL) */
    size_t           error_tam;
} SolarOpciones;

int solar_copiar(const char *const *origenes, size_t cantidad, const char *destino,
                 const SolarOpciones *opciones);
int solar_mover(const char *const *origenes, size_t cantidad, const char *destino,
                const SolarOpciones *opciones);
int solar_eliminar(const char *const *rutas, size_t cantidad, const SolarOpciones *opciones);

int solar_renombrar(const char *ruta, const char *nombre_nuevo);
int solar_crear_carpeta(const char *dir, const char *nombre);
int solar_crear_archivo(const char *dir, const char *nombre);
/* 0 si el nombre sirve; EINVAL si está vacío, es "." / ".." o tiene '/'. */
int solar_validar_nombre(const char *nombre);
/* Nombre que no existe en dir: "foto (2).png", o "foto (copia).png" si
 * es_copia. Escribe solo el nombre (sin la carpeta). */
int solar_nombre_libre(const char *dir, const char *nombre, int es_copia,
                       char *salida, size_t tam);

/* Abrir cosas con otros programas (no bloquea; el proceso queda suelto). */
int solar_abrir(const char *ruta);                                   /* xdg-open */
int solar_abrir_con(const char *comando, const char *const *rutas, size_t cantidad);
int solar_abrir_terminal(const char *terminal, const char *dir);     /* "" = detectar */
int solar_detectar_terminal(char *salida, size_t tam);
int solar_lanzar(const char *const *argv, const char *dir_trabajo);

/* ===================================================================== */
/*  Papelera (especificación de freedesktop.org)                         */
/* ===================================================================== */

typedef struct {
    char    *nombre;          /* nombre original */
    char    *ruta_original;   /* dónde estaba */
    char    *ruta;            /* dónde está ahora (dentro de Trash/files) */
    int64_t  fecha;           /* cuándo se borró (0 si no se sabe) */
    uint64_t tamano;
    uint8_t  es_carpeta;
    uint8_t  categoria;
    uint8_t  _relleno[6];
} SolarElementoPapelera;

typedef struct {
    SolarElementoPapelera *elementos;
    size_t                 cantidad;
} SolarPapelera;

int  solar_papelera_enviar(const char *const *rutas, size_t cantidad, const SolarOpciones *opciones);
int  solar_papelera_listar(SolarPapelera *salida);
void solar_papelera_liberar(SolarPapelera *papelera);
/* Restaura un elemento (por su ruta dentro de la papelera). Si su lugar
 * original ya está ocupado, se restaura como "nombre (2)". */
int  solar_papelera_restaurar(const char *ruta, char *restaurado, size_t restaurado_tam,
                              char *error, size_t error_tam);
int  solar_papelera_borrar(const char *ruta);
int  solar_papelera_vaciar(const SolarOpciones *opciones);
int  solar_papelera_vacia(void);
int  solar_papelera_dir(char *salida, size_t tam);

/* ===================================================================== */
/*  Dispositivos (memorias USB, discos)                                  */
/* ===================================================================== */

typedef struct {
    char    *dispositivo;       /* /dev/sdb1 */
    char    *nombre;            /* etiqueta o nombre amigable */
    char    *punto_montaje;     /* NULL si no está montado */
    char    *sistema_archivos;  /* vfat, ext4… (puede ser NULL) */
    uint64_t tamano;            /* bytes totales */
    uint64_t libre;             /* bytes disponibles (si está montado) */
    uint8_t  extraible;         /* USB, tarjeta SD… */
    uint8_t  montado;
    uint8_t  solo_lectura;
    uint8_t  _relleno[5];
} SolarDispositivo;

typedef struct {
    SolarDispositivo *dispositivos;
    size_t            cantidad;
} SolarDispositivos;

int  solar_dispositivos_listar(int incluir_internos, SolarDispositivos *salida);
void solar_dispositivos_liberar(SolarDispositivos *dispositivos);
int  solar_dispositivo_montar(const char *dispositivo, char *error, size_t error_tam);
int  solar_dispositivo_desmontar(const char *dispositivo, int expulsar,
                                 char *error, size_t error_tam);

typedef struct {
    uint64_t total;
    uint64_t libre;
    uint64_t disponible;
} SolarEspacio;

int solar_espacio(const char *ruta, SolarEspacio *espacio);

/* ===================================================================== */
/*  Vigilante (avisa cuando algo cambia)                                 */
/* ===================================================================== */

enum {
    SOLAR_CAMBIO_CARPETA = 1,
    SOLAR_CAMBIO_MONTAJES = 2,
    SOLAR_CAMBIO_PAPELERA = 4
};

typedef struct SolarVigilante SolarVigilante;

SolarVigilante *solar_vigilante_crear(void);
/* Observa una carpeta (reemplaza la anterior). NULL deja de observar. */
int  solar_vigilante_observar(SolarVigilante *v, const char *ruta);
/* Espera hasta 'milisegundos'. Devuelve una máscara SOLAR_CAMBIO_*, 0 si
 * no pasó nada, o -1 si hubo un error. */
int  solar_vigilante_esperar(SolarVigilante *v, int milisegundos);
void solar_vigilante_destruir(SolarVigilante *v);

/* ===================================================================== */
/*  Carpetas personalizadas (color + logotipo)                           */
/* ===================================================================== */

typedef struct {
    uint32_t color;             /* 0xRRGGBB */
    uint32_t color_icono;       /* 0xRRGGBB */
    uint8_t  tiene_color;
    uint8_t  tiene_color_icono;
    uint8_t  _relleno[2];
    char     icono[48];         /* "mando", "musica", "txt:★"… o "" */
} SolarEstiloCarpeta;

typedef struct SolarCarpetas SolarCarpetas;

SolarCarpetas *solar_carpetas_abrir(const char *archivo);
/* 1 si la carpeta tiene estilo (y lo copia en 'estilo'), 0 si no. */
int    solar_carpetas_obtener(const SolarCarpetas *db, const char *ruta, SolarEstiloCarpeta *estilo);
int    solar_carpetas_poner(SolarCarpetas *db, const char *ruta, const SolarEstiloCarpeta *estilo);
int    solar_carpetas_quitar(SolarCarpetas *db, const char *ruta);
/* Olvida la carpeta y todo lo que hay dentro (al eliminarla para siempre). */
int    solar_carpetas_olvidar_arbol(SolarCarpetas *db, const char *ruta);
/* La carpeta cambió de lugar o de nombre: mueve su estilo y el de sus hijas. */
int    solar_carpetas_renombrar(SolarCarpetas *db, const char *vieja, const char *nueva);
/* Copia el estilo de una carpeta (y sus hijas) a otra ruta. */
int    solar_carpetas_copiar(SolarCarpetas *db, const char *origen, const char *destino);
size_t solar_carpetas_cantidad(const SolarCarpetas *db);
void   solar_carpetas_cerrar(SolarCarpetas *db);

/* ===================================================================== */
/*  Configuración (Lua)                                                  */
/* ===================================================================== */

enum { SOLAR_POS_ARRIBA = 0, SOLAR_POS_ABAJO = 1 };
enum { SOLAR_LADO_IZQUIERDA = 0, SOLAR_LADO_DERECHA = 1 };
enum { SOLAR_VISTA_CUADRICULA = 0, SOLAR_VISTA_LISTA = 1 };

typedef struct {
    char *nombre;
    char *ruta;        /* puede empezar con "~" */
    char *icono;       /* nombre de logotipo o "" */
} SolarLugar;

typedef struct {
    char *accion;      /* "copiar", "nueva_carpeta"… */
    char *teclas;      /* "Ctrl+C", "Alt+Left", "" = desactivado */
} SolarAtajo;

typedef struct {
    /* Apariencia */
    char   *tema;
    int32_t transparencia;
    double  opacidad;               /* 0.30 – 1.00 */
    double  escala;                 /* 0.75 – 2.00 */
    char   *fuente;                 /* "" integrada, "sistema" o ruta a .ttf/.otf */
    int32_t barra_titulo_integrada;
    /* Distribución */
    int32_t barra_ubicacion;        /* SOLAR_POS_* */
    int32_t barra_lateral;          /* SOLAR_LADO_* */
    int32_t mostrar_barra_lateral;
    /* Archivos */
    int32_t vista;                  /* SOLAR_VISTA_* */
    int32_t tamano_iconos;          /* 40 – 160 */
    int32_t mostrar_ocultos;
    int32_t carpetas_primero;
    int32_t orden;                  /* SOLAR_ORDEN_* */
    int32_t orden_descendente;
    int32_t un_clic;
    /* Comportamiento */
    int32_t confirmar_papelera;
    int32_t confirmar_eliminar;
    char   *terminal;               /* "" = detectar */
    /* Dispositivos */
    int32_t mostrar_usb;
    int32_t mostrar_discos_internos;
    /* Listas */
    SolarLugar *lugares;
    size_t      n_lugares;
    SolarAtajo *atajos;
    size_t      n_atajos;
} SolarConfig;

/* Llena 'c' con los valores por defecto (memoria del núcleo). */
void solar_config_defecto(SolarConfig *c);
/* 'c' debe venir de solar_config_defecto(). Devuelve 0 si se leyó bien,
 * ENOENT si no existe el archivo (c queda por defecto) o EINVAL si el Lua
 * tiene un error (c queda por defecto y 'avisos' dice por qué). Los
 * valores raros no son error: se ignoran y se explican en 'avisos'. */
int  solar_config_cargar(const char *archivo, SolarConfig *c, char *avisos, size_t avisos_tam);
/* Solo lee 'c' (puede venir de cualquier lado). */
int  solar_config_guardar(const char *archivo, const SolarConfig *c);
void solar_config_liberar(SolarConfig *c);

/* Tabla de atajos por defecto (estática, no liberar). */
size_t solar_atajos_defecto(const SolarAtajo **tabla);

/* ===================================================================== */
/*  Temas                                                                */
/* ===================================================================== */

enum {
    SOLAR_COLOR_FONDO = 0,
    SOLAR_COLOR_SUPERFICIE,
    SOLAR_COLOR_ELEVADO,
    SOLAR_COLOR_BORDE,
    SOLAR_COLOR_TEXTO,
    SOLAR_COLOR_TEXTO_TENUE,
    SOLAR_COLOR_ACENTO,
    SOLAR_COLOR_TEXTO_ACENTO,
    SOLAR_COLOR_SELECCION,
    SOLAR_COLOR_PELIGRO,
    SOLAR_COLOR_EXITO,
    SOLAR_COLOR_AVISO,
    SOLAR_COLOR_CARPETA,
    SOLAR_COLORES_TOTAL
};

typedef struct {
    char    *id;
    char    *nombre;
    char    *ruta;          /* NULL en los temas integrados */
    int32_t  oscuro;
    int32_t  integrado;
    uint32_t colores[SOLAR_COLORES_TOTAL];   /* 0xRRGGBB */
} SolarTema;

typedef struct {
    SolarTema *temas;
    size_t     cantidad;
} SolarTemas;

/* Temas integrados + los .lua de dir_usuario (puede ser NULL). */
int  solar_temas_cargar(const char *dir_usuario, SolarTemas *salida, char *avisos, size_t avisos_tam);
void solar_temas_liberar(SolarTemas *temas);
int  solar_tema_guardar(const char *archivo, const SolarTema *tema);
/* Clave Lua de cada color ("fondo", "acento"…). */
const char *solar_color_clave(int indice);

#ifdef __cplusplus
}
#endif

#endif /* SOLAR_H */
