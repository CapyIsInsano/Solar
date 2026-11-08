/*
 * ☀ Solar — utilidades internas del núcleo (no son API pública)
 * Licencia: GPL-3.0-or-later
 */
#ifndef SOLAR_INTERNO_H
#define SOLAR_INTERNO_H

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "solar.h"

#include <errno.h>
#include <limits.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>

#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

#define SOLAR_LEN(a) (sizeof(a) / sizeof((a)[0]))

/* ---------- Cadenas dinámicas ---------- */

typedef struct {
    char  *datos;
    size_t largo;
    size_t capacidad;
    int    sin_memoria;
} Cadena;

void cadena_iniciar(Cadena *c);
void cadena_agregar(Cadena *c, const char *texto);
void cadena_agregar_n(Cadena *c, const char *texto, size_t n);
void cadena_printf(Cadena *c, const char *formato, ...) __attribute__((format(printf, 2, 3)));
/* Agrega un literal de cadena Lua entre comillas, escapado. */
void cadena_agregar_lua(Cadena *c, const char *texto);
void cadena_liberar(Cadena *c);

/* ---------- Memoria y texto ---------- */

char *solar_strdup(const char *s);          /* NULL -> NULL */
char *solar_strndup(const char *s, size_t n);
/* Copia segura: siempre termina en '\0'. Devuelve 0 o ENAMETOOLONG. */
int   copiar_texto(char *destino, size_t tam, const char *origen);
void  poner_error(char *buf, size_t tam, const char *formato, ...) __attribute__((format(printf, 3, 4)));
/* Añade una línea a un búfer de avisos (sin desbordar). */
void  agregar_aviso(char *buf, size_t tam, const char *formato, ...) __attribute__((format(printf, 3, 4)));
/* snprintf que avisa si no cupo: devuelve 0 o ENAMETOOLONG. */
int   formatear(char *buf, size_t tam, const char *formato, ...) __attribute__((format(printf, 3, 4)));
int   termina_con(const char *s, const char *sufijo);
void  a_minusculas(char *s);

/* ---------- Rutas ---------- */

char       *ruta_unir(const char *dir, const char *nombre);   /* malloc */
const char *ruta_base(const char *ruta);                      /* puntero dentro de ruta */
char       *ruta_padre(const char *ruta);                     /* malloc */
void        ruta_normalizar(char *ruta);                      /* en el sitio */
int         ruta_es_dentro_de(const char *hija, const char *padre); /* hija == padre o hija/… */
int         existe(const char *ruta);                         /* lstat == 0 */
int         es_carpeta(const char *ruta);                     /* stat + S_ISDIR */
int         crear_dirs(const char *ruta, mode_t modo);        /* mkdir -p */
int         obtener_hogar(char *salida, size_t tam);
int         escribir_atomico(const char *ruta, const char *datos, size_t largo);
/* Lee todo un archivo pequeño (máx. 'limite' bytes). malloc, termina en '\0'. */
char       *leer_archivo(const char *ruta, size_t limite, size_t *largo);
/* Directorio XDG del usuario (DESKTOP, DOCUMENTS…) leyendo user-dirs.dirs. */
int         xdg_dir_usuario(const char *clave, char *salida, size_t tam);

/* ---------- Procesos ---------- */

/* Busca un programa en $PATH. */
int buscar_en_path(const char *programa, char *salida, size_t tam);
/* Divide un comando estilo shell ("kitty --hold 'a b'") en argv (malloc). */
int dividir_comando(const char *comando, char ***argv, size_t *n);
void liberar_argv(char **argv, size_t n);
/* Ejecuta y espera (máx. timeout_ms), capturando stdout+stderr. Devuelve
 * el código de salida (>= 0), o -errno si no se pudo ejecutar. */
int ejecutar_y_esperar(const char *const *argv, char *salida, size_t tam, int timeout_ms);

/* ---------- Árboles de archivos ---------- */

/* Elimina recursivamente (sin seguir enlaces). Llama a 'cada' por elemento
 * borrado; si 'cada' devuelve != 0 se detiene con ECANCELED. */
typedef int (*CadaElementoFn)(void *datos, const char *ruta);
int eliminar_arbol(const char *ruta, CadaElementoFn cada, void *datos);
/* Cuenta bytes y elementos de un árbol (sin seguir enlaces). */
int contar_arbol(const char *ruta, uint64_t *bytes, uint64_t *elementos);

/* ---------- Montajes (/proc/self/mountinfo) ---------- */

typedef struct {
    unsigned mayor, menor;
    char    *punto;
    char    *tipo_fs;
    char    *fuente;
    int      solo_lectura;
} Montaje;

int  leer_montajes(Montaje **salida, size_t *cantidad);
void liberar_montajes(Montaje *m, size_t cantidad);

/* ---------- Lua (entorno seguro) ---------- */

struct lua_State;
/* Crea un estado Lua aislado (sin io, sin os.execute, memoria limitada). */
struct lua_State *lua_seguro_crear(void);
/* Carga y ejecuta un archivo .lua (solo texto). Deja en la pila la tabla
 * de configuración (lo que devolvió el archivo o, si no devolvió nada,
 * la tabla global). Devuelve 0, ENOENT o EINVAL (con el error en 'avisos'). */
int lua_seguro_ejecutar(struct lua_State *L, const char *archivo, char *avisos, size_t avisos_tam);
void lua_seguro_cerrar(struct lua_State *L);
/* "#RRGGBB", "#RGB" o número 0xRRGGBB. Devuelve 1 si es un color válido. */
int lua_leer_color(struct lua_State *L, int indice, uint32_t *color);

#endif /* SOLAR_INTERNO_H */
