/*
 * ☀ Solar — configuración en Lua (~/.config/solar/config.lua)
 *
 * Solar reescribe este archivo cuando cambias algo en Ajustes, pero es Lua
 * normal y puedes editarlo a mano. Lo que no se entienda se ignora y se
 * explica en los avisos (Solar los muestra al abrir).
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <lauxlib.h>
#include <lua.h>

/* ===================================================================== */
/*  Valores por defecto                                                  */
/* ===================================================================== */

static const SolarAtajo ATAJOS[] = {
    { "atras", "Alt+Left" },
    { "adelante", "Alt+Right" },
    { "subir", "Alt+Up" },
    { "inicio", "Alt+Home" },
    { "recargar", "F5" },
    { "ubicacion", "Ctrl+L" },
    { "buscar", "Ctrl+F" },
    { "ocultos", "Ctrl+H" },
    { "nueva_carpeta", "Ctrl+Shift+N" },
    { "nuevo_archivo", "Ctrl+Alt+N" },
    { "renombrar", "F2" },
    { "copiar", "Ctrl+C" },
    { "cortar", "Ctrl+X" },
    { "pegar", "Ctrl+V" },
    { "duplicar", "Ctrl+Shift+D" },
    { "seleccionar_todo", "Ctrl+A" },
    { "papelera", "Delete" },
    { "eliminar", "Shift+Delete" },
    { "propiedades", "Alt+Enter" },
    { "personalizar", "Ctrl+E" },
    { "agregar_lugar", "Ctrl+D" },
    { "vista_cuadricula", "Ctrl+1" },
    { "vista_lista", "Ctrl+2" },
    { "vista_mosaicos", "Ctrl+3" },
    { "vista_orbita", "Ctrl+4" },
    { "acercar", "Ctrl+Plus" },
    { "alejar", "Ctrl+Minus" },
    { "barra_lateral", "F9" },
    { "terminal", "F4" },
    { "ajustes", "Ctrl+Comma" },
    { "nueva_pestana", "Ctrl+T" },
    { "cerrar_pestana", "Ctrl+W" },
    { "siguiente_pestana", "Ctrl+Tab" },
    { "anterior_pestana", "Ctrl+Shift+Tab" },
    { "reabrir_pestana", "Ctrl+Shift+T" },
    { "espacios", "Ctrl+Shift+E" },
    { "nueva_ventana", "Ctrl+N" },
    { "cerrar", "Ctrl+Q" },
};

size_t solar_atajos_defecto(const SolarAtajo **tabla)
{
    if (tabla) *tabla = ATAJOS;
    return SOLAR_LEN(ATAJOS);
}

static void agregar_lugar(SolarConfig *c, size_t *cap, const char *nombre, const char *ruta, const char *icono)
{
    if (c->n_lugares == *cap) {
        size_t nuevo = *cap ? *cap * 2 : 8;
        SolarLugar *q = realloc(c->lugares, nuevo * sizeof *q);
        if (!q) return;
        c->lugares = q;
        *cap = nuevo;
    }
    SolarLugar *l = &c->lugares[c->n_lugares];
    l->nombre = solar_strdup(nombre);
    l->ruta = solar_strdup(ruta);
    l->icono = solar_strdup(icono ? icono : "");
    if (!l->nombre || !l->ruta || !l->icono) {
        free(l->nombre);
        free(l->ruta);
        free(l->icono);
        return;
    }
    c->n_lugares++;
}

/* "/home/capy/Música" -> "~/Música" */
static void contraer_hogar(const char *ruta, char *salida, size_t tam)
{
    char hogar[PATH_MAX];
    obtener_hogar(hogar, sizeof hogar);
    size_t n = strlen(hogar);
    if (n > 1 && !strncmp(ruta, hogar, n) && (ruta[n] == '/' || ruta[n] == '\0'))
        snprintf(salida, tam, "~%s", ruta + n);
    else
        copiar_texto(salida, tam, ruta);
}

static void lugar_xdg(SolarConfig *c, size_t *cap, const char *clave, const char *nombre,
                      const char *icono, const char *const *candidatos)
{
    char ruta[PATH_MAX], corta[PATH_MAX];
    if (xdg_dir_usuario(clave, ruta, sizeof ruta) == 0 && es_carpeta(ruta)) {
        contraer_hogar(ruta, corta, sizeof corta);
        agregar_lugar(c, cap, nombre, corta, icono);
        return;
    }
    char hogar[PATH_MAX];
    obtener_hogar(hogar, sizeof hogar);
    for (size_t i = 0; candidatos[i]; i++) {
        if (formatear(ruta, sizeof ruta, "%s/%s", hogar, candidatos[i])) continue;
        if (es_carpeta(ruta)) {
            snprintf(corta, sizeof corta, "~/%s", candidatos[i]);
            agregar_lugar(c, cap, nombre, corta, icono);
            return;
        }
    }
}

void solar_ajustes_vista_defecto(SolarAjustesVista *a)
{
    a->tamano_iconos = 72;
    a->mostrar_ocultos = 0;
    a->carpetas_primero = 1;
    a->orden = SOLAR_ORDEN_NOMBRE;
    a->orden_descendente = 0;
    a->orbita_girar = 1;
    a->orbita_velocidad = 1.0;
    a->orbita_orden = SOLAR_ORBITA_GRANDE_PRIMERO;
    a->orbita_electricidad = 1;
}

void solar_config_defecto(SolarConfig *c)
{
    memset(c, 0, sizeof *c);
    c->tema = solar_strdup("sol");
    c->transparencia = 0;
    c->opacidad = 0.85;
    c->escala = 1.0;
    c->fuente = solar_strdup("");
    c->barra_titulo_integrada = 1;
    c->barra_ubicacion = SOLAR_POS_ARRIBA;
    c->barra_lateral = SOLAR_LADO_IZQUIERDA;
    c->mostrar_barra_lateral = 1;
    c->vista = SOLAR_VISTA_CUADRICULA;
    solar_ajustes_vista_defecto(&c->archivos);
    c->un_clic = 0;
    c->confirmar_papelera = 0;
    c->confirmar_eliminar = 1;
    c->terminal = solar_strdup("");
    c->restaurar_sesion = 1;
    c->mostrar_usb = 1;
    c->mostrar_discos_internos = 0;
    c->sonidos = 1;
    c->volumen = 0.6;
    c->animaciones = 1;
    c->buscar_actualizaciones = 1;
    c->repo_actualizaciones = solar_strdup("CapyIsInsano/Solar");

    size_t cap = 0;
    agregar_lugar(c, &cap, "Carpeta personal", "~", "casa");
    static const char *const ESCRITORIO[] = { "Escritorio", "Desktop", NULL };
    static const char *const DOCUMENTOS[] = { "Documentos", "Documents", NULL };
    static const char *const DESCARGAS[] = { "Descargas", "Downloads", NULL };
    static const char *const MUSICA[] = { "Música", "Musica", "Music", NULL };
    static const char *const IMAGENES[] = { "Imágenes", "Imagenes", "Pictures", NULL };
    static const char *const VIDEOS[] = { "Vídeos", "Videos", NULL };
    lugar_xdg(c, &cap, "DESKTOP", "Escritorio", "escritorio", ESCRITORIO);
    lugar_xdg(c, &cap, "DOCUMENTS", "Documentos", "documento", DOCUMENTOS);
    lugar_xdg(c, &cap, "DOWNLOAD", "Descargas", "descarga", DESCARGAS);
    lugar_xdg(c, &cap, "MUSIC", "Música", "musica", MUSICA);
    lugar_xdg(c, &cap, "PICTURES", "Imágenes", "imagen", IMAGENES);
    lugar_xdg(c, &cap, "VIDEOS", "Videos", "video", VIDEOS);

    c->atajos = calloc(SOLAR_LEN(ATAJOS), sizeof *c->atajos);
    if (c->atajos) {
        for (size_t i = 0; i < SOLAR_LEN(ATAJOS); i++) {
            c->atajos[i].accion = solar_strdup(ATAJOS[i].accion);
            c->atajos[i].teclas = solar_strdup(ATAJOS[i].teclas);
            if (!c->atajos[i].accion || !c->atajos[i].teclas) {
                free(c->atajos[i].accion);
                free(c->atajos[i].teclas);
                break;
            }
            c->n_atajos++;
        }
    }
}

void solar_config_liberar(SolarConfig *c)
{
    if (!c) return;
    free(c->tema);
    free(c->fuente);
    free(c->terminal);
    free(c->repo_actualizaciones);
    for (size_t i = 0; i < c->n_lugares; i++) {
        free(c->lugares[i].nombre);
        free(c->lugares[i].ruta);
        free(c->lugares[i].icono);
    }
    free(c->lugares);
    for (size_t i = 0; i < c->n_atajos; i++) {
        free(c->atajos[i].accion);
        free(c->atajos[i].teclas);
    }
    free(c->atajos);
    memset(c, 0, sizeof *c);
}

/* ===================================================================== */
/*  Lectura                                                              */
/* ===================================================================== */

void lector_bool(Lector *l, const char *clave, int32_t *destino)
{
    lua_getfield(l->L, l->tabla, clave);
    if (lua_isboolean(l->L, -1)) *destino = lua_toboolean(l->L, -1);
    else if (!lua_isnil(l->L, -1)) agregar_aviso(l->avisos, l->tam, "«%s» debe ser true o false.", clave);
    lua_pop(l->L, 1);
}

void lector_numero(Lector *l, const char *clave, double *destino, double min, double max)
{
    lua_getfield(l->L, l->tabla, clave);
    if (lua_type(l->L, -1) == LUA_TNUMBER) {
        double v = lua_tonumber(l->L, -1);
        if (v >= min && v <= max) *destino = v;
        else agregar_aviso(l->avisos, l->tam, "«%s» debe estar entre %g y %g.", clave, min, max);
    } else if (!lua_isnil(l->L, -1)) {
        agregar_aviso(l->avisos, l->tam, "«%s» debe ser un número.", clave);
    }
    lua_pop(l->L, 1);
}

void lector_entero(Lector *l, const char *clave, int32_t *destino, int min, int max)
{
    double v = *destino;
    lector_numero(l, clave, &v, min, max);
    *destino = (int32_t)(v + 0.5);
}

void lector_texto(Lector *l, const char *clave, char **destino)
{
    lua_getfield(l->L, l->tabla, clave);
    if (lua_type(l->L, -1) == LUA_TSTRING) {
        char *nuevo = solar_strdup(lua_tostring(l->L, -1));
        if (nuevo) {
            free(*destino);
            *destino = nuevo;
        }
    } else if (!lua_isnil(l->L, -1)) {
        agregar_aviso(l->avisos, l->tam, "«%s» debe ser texto (entre comillas).", clave);
    }
    lua_pop(l->L, 1);
}

void lector_opcion(Lector *l, const char *clave, int32_t *destino, const Opcion *opciones, size_t n)
{
    lua_getfield(l->L, l->tabla, clave);
    if (lua_type(l->L, -1) == LUA_TSTRING) {
        const char *v = lua_tostring(l->L, -1);
        int encontrada = 0;
        for (size_t i = 0; i < n; i++) {
            if (!strcmp(v, opciones[i].nombre)) {
                *destino = opciones[i].valor;
                encontrada = 1;
                break;
            }
        }
        if (!encontrada) {
            char lista[256] = "";
            for (size_t i = 0, puestas = 0; i < n && puestas < 4; i++) {
                int repetido = 0;
                for (size_t j = 0; j < i; j++)
                    if (opciones[j].valor == opciones[i].valor) repetido = 1;
                if (repetido) continue;
                size_t u = strlen(lista);
                snprintf(lista + u, sizeof lista - u, "%s\"%s\"", puestas ? ", " : "", opciones[i].nombre);
                puestas++;
            }
            agregar_aviso(l->avisos, l->tam, "«%s» no entiende \"%s\". Opciones: %s.", clave, v, lista);
        }
    } else if (!lua_isnil(l->L, -1)) {
        agregar_aviso(l->avisos, l->tam, "«%s» debe ser texto (entre comillas).", clave);
    }
    lua_pop(l->L, 1);
}

static void leer_lugares(Lector *l, SolarConfig *c)
{
    lua_State *L = l->L;
    lua_getfield(L, l->tabla, "lugares");
    if (lua_isnil(L, -1)) { lua_pop(L, 1); return; }
    if (!lua_istable(L, -1)) {
        agregar_aviso(l->avisos, l->tam, "«lugares» debe ser una lista: { { nombre = \"…\", ruta = \"…\" }, … }.");
        lua_pop(L, 1);
        return;
    }
    /* Reemplaza los lugares por defecto */
    for (size_t i = 0; i < c->n_lugares; i++) {
        free(c->lugares[i].nombre);
        free(c->lugares[i].ruta);
        free(c->lugares[i].icono);
    }
    free(c->lugares);
    c->lugares = NULL;
    c->n_lugares = 0;
    size_t cap = 0;
    lua_Integer total = (lua_Integer)lua_rawlen(L, -1);
    for (lua_Integer i = 1; i <= total && i <= 256; i++) {
        lua_rawgeti(L, -1, i);
        if (lua_type(L, -1) == LUA_TSTRING) {
            const char *ruta = lua_tostring(L, -1);
            agregar_lugar(c, &cap, ruta_base(ruta), ruta, "");
        } else if (lua_istable(L, -1)) {
            lua_getfield(L, -1, "nombre");
            lua_getfield(L, -2, "ruta");
            lua_getfield(L, -3, "icono");
            const char *nombre = lua_type(L, -3) == LUA_TSTRING ? lua_tostring(L, -3) : NULL;
            const char *ruta = lua_type(L, -2) == LUA_TSTRING ? lua_tostring(L, -2) : NULL;
            const char *icono = lua_type(L, -1) == LUA_TSTRING ? lua_tostring(L, -1) : "";
            if (ruta && *ruta) agregar_lugar(c, &cap, nombre && *nombre ? nombre : ruta_base(ruta), ruta, icono);
            else agregar_aviso(l->avisos, l->tam, "El lugar número %d no tiene «ruta».", (int)i);
            lua_pop(L, 3);
        } else {
            agregar_aviso(l->avisos, l->tam, "El lugar número %d no se entiende.", (int)i);
        }
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
}

static void leer_atajos(Lector *l, SolarConfig *c)
{
    lua_State *L = l->L;
    lua_getfield(L, l->tabla, "atajos");
    if (lua_isnil(L, -1)) { lua_pop(L, 1); return; }
    if (!lua_istable(L, -1)) {
        agregar_aviso(l->avisos, l->tam, "«atajos» debe ser una tabla: { copiar = \"Ctrl+C\", … }.");
        lua_pop(L, 1);
        return;
    }
    lua_pushnil(L);
    while (lua_next(L, -2)) {
        if (lua_type(L, -2) != LUA_TSTRING) { lua_pop(L, 1); continue; }
        const char *accion = lua_tostring(L, -2);
        SolarAtajo *a = NULL;
        for (size_t i = 0; i < c->n_atajos; i++)
            if (!strcmp(c->atajos[i].accion, accion)) { a = &c->atajos[i]; break; }
        if (!a) {
            agregar_aviso(l->avisos, l->tam, "Atajo desconocido «%s» (se ignora).", accion);
        } else if (lua_type(L, -1) == LUA_TSTRING) {
            char *nuevo = solar_strdup(lua_tostring(L, -1));
            if (nuevo) { free(a->teclas); a->teclas = nuevo; }
        } else if (lua_isboolean(L, -1) && !lua_toboolean(L, -1)) {
            char *nuevo = solar_strdup("");
            if (nuevo) { free(a->teclas); a->teclas = nuevo; }
        } else {
            agregar_aviso(l->avisos, l->tam, "El atajo «%s» debe ser texto, como \"Ctrl+Shift+N\", o false.", accion);
        }
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
}

/* Opciones que se escriben como texto */
const Opcion OPCIONES_VISTA[] = {
    { "cuadricula", SOLAR_VISTA_CUADRICULA }, { "lista", SOLAR_VISTA_LISTA },
    { "mosaicos", SOLAR_VISTA_MOSAICOS }, { "orbita", SOLAR_VISTA_ORBITA },
    { "cuadrícula", SOLAR_VISTA_CUADRICULA }, { "órbita", SOLAR_VISTA_ORBITA },
    { "grid", SOLAR_VISTA_CUADRICULA }, { "list", SOLAR_VISTA_LISTA },
    { "tiles", SOLAR_VISTA_MOSAICOS }, { "orbit", SOLAR_VISTA_ORBITA },
};
const size_t N_OPCIONES_VISTA = SOLAR_LEN(OPCIONES_VISTA);
const char *const NOMBRES_VISTA[4] = { "cuadricula", "lista", "mosaicos", "orbita" };

static const Opcion ORDENES[] = { { "nombre", SOLAR_ORDEN_NOMBRE }, { "tamano", SOLAR_ORDEN_TAMANO },
                                  { "fecha", SOLAR_ORDEN_FECHA }, { "tipo", SOLAR_ORDEN_TIPO },
                                  { "tamaño", SOLAR_ORDEN_TAMANO }, { "name", SOLAR_ORDEN_NOMBRE },
                                  { "size", SOLAR_ORDEN_TAMANO }, { "date", SOLAR_ORDEN_FECHA },
                                  { "type", SOLAR_ORDEN_TIPO } };
static const char *const NOMBRES_ORDEN[4] = { "nombre", "tamano", "fecha", "tipo" };

static const Opcion ORDENES_ORBITA[] = {
    { "grande_primero", SOLAR_ORBITA_GRANDE_PRIMERO }, { "chico_primero", SOLAR_ORBITA_CHICO_PRIMERO },
    { "nombre", SOLAR_ORBITA_NOMBRE }, { "tipo", SOLAR_ORBITA_TIPO }, { "fecha", SOLAR_ORBITA_FECHA },
    { "aleatorio", SOLAR_ORBITA_ALEATORIO }, { "grandes", SOLAR_ORBITA_GRANDE_PRIMERO },
    { "chicos", SOLAR_ORBITA_CHICO_PRIMERO }, { "azar", SOLAR_ORBITA_ALEATORIO },
    { "random", SOLAR_ORBITA_ALEATORIO },
};
static const char *const NOMBRES_ORBITA[6] = { "grande_primero", "chico_primero", "nombre", "tipo", "fecha", "aleatorio" };

const char *nombre_orden(int orden) { return NOMBRES_ORDEN[orden >= 0 && orden <= 3 ? orden : 0]; }
const char *nombre_orden_orbita(int orden) { return NOMBRES_ORBITA[orden >= 0 && orden <= 5 ? orden : 0]; }

void leer_ajustes_vista(Lector *l, SolarAjustesVista *a)
{
    lector_entero(l, "tamano_iconos", &a->tamano_iconos, 40, 160);
    lector_bool(l, "mostrar_ocultos", &a->mostrar_ocultos);
    lector_bool(l, "carpetas_primero", &a->carpetas_primero);
    lector_opcion(l, "orden", &a->orden, ORDENES, SOLAR_LEN(ORDENES));
    lector_bool(l, "orden_descendente", &a->orden_descendente);
    lector_bool(l, "orbita_girar", &a->orbita_girar);
    lector_numero(l, "orbita_velocidad", &a->orbita_velocidad, 0.10, 4.00);
    lector_opcion(l, "orbita_orden", &a->orbita_orden, ORDENES_ORBITA, SOLAR_LEN(ORDENES_ORBITA));
    lector_bool(l, "orbita_electricidad", &a->orbita_electricidad);
}

static const char *const CLAVES_CONOCIDAS[] = {
    "tema", "transparencia", "opacidad", "escala", "fuente", "barra_titulo_integrada",
    "barra_ubicacion", "barra_lateral", "mostrar_barra_lateral", "vista", "tamano_iconos",
    "mostrar_ocultos", "carpetas_primero", "orden", "orden_descendente", "un_clic",
    "confirmar_papelera", "confirmar_eliminar", "terminal", "restaurar_sesion", "mostrar_usb",
    "mostrar_discos_internos", "sonidos", "volumen", "animaciones",
    "buscar_actualizaciones", "repo_actualizaciones", "orbita_girar", "orbita_velocidad",
    "orbita_orden", "orbita_electricidad", "lugares", "atajos",
};

static void avisar_desconocidas(Lector *l)
{
    lua_State *L = l->L;
    lua_pushnil(L);
    int n = 0;
    while (lua_next(L, l->tabla)) {
        lua_pop(L, 1);
        if (lua_type(L, -1) != LUA_TSTRING) continue;
        const char *k = lua_tostring(L, -1);
        int conocida = 0;
        for (size_t i = 0; i < SOLAR_LEN(CLAVES_CONOCIDAS); i++)
            if (!strcmp(k, CLAVES_CONOCIDAS[i])) { conocida = 1; break; }
        if (!conocida && n++ < 8) agregar_aviso(l->avisos, l->tam, "No conozco la opción «%s» (¿un error de dedo?).", k);
    }
}

int solar_config_cargar(const char *archivo, SolarConfig *c, char *avisos, size_t avisos_tam)
{
    if (avisos && avisos_tam) avisos[0] = '\0';
    if (!existe(archivo)) return ENOENT;
    lua_State *L = lua_seguro_crear();
    if (!L) return ENOMEM;
    int r = lua_seguro_ejecutar(L, archivo, avisos, avisos_tam);
    if (r) {
        lua_seguro_cerrar(L);
        return r;
    }
    Lector l = { L, lua_gettop(L), avisos, avisos_tam };
    lua_pushglobaltable(L);
    int es_global = lua_rawequal(L, -1, l.tabla);
    lua_pop(L, 1);

    static const Opcion POSICIONES[] = { { "arriba", SOLAR_POS_ARRIBA }, { "abajo", SOLAR_POS_ABAJO },
                                         { "top", SOLAR_POS_ARRIBA }, { "bottom", SOLAR_POS_ABAJO } };
    static const Opcion LADOS[] = { { "izquierda", SOLAR_LADO_IZQUIERDA }, { "derecha", SOLAR_LADO_DERECHA },
                                    { "left", SOLAR_LADO_IZQUIERDA }, { "right", SOLAR_LADO_DERECHA } };

    lector_texto(&l, "tema", &c->tema);
    if (c->tema && strcmp(solar_tema_id_actual(c->tema), c->tema)) {
        char *nuevo = solar_strdup(solar_tema_id_actual(c->tema));
        if (nuevo) { free(c->tema); c->tema = nuevo; }
    }
    lector_bool(&l, "transparencia", &c->transparencia);
    lector_numero(&l, "opacidad", &c->opacidad, 0.30, 1.00);
    lector_numero(&l, "escala", &c->escala, 0.75, 2.00);
    lector_texto(&l, "fuente", &c->fuente);
    lector_bool(&l, "barra_titulo_integrada", &c->barra_titulo_integrada);
    lector_opcion(&l, "barra_ubicacion", &c->barra_ubicacion, POSICIONES, SOLAR_LEN(POSICIONES));
    lector_opcion(&l, "barra_lateral", &c->barra_lateral, LADOS, SOLAR_LEN(LADOS));
    lector_bool(&l, "mostrar_barra_lateral", &c->mostrar_barra_lateral);
    lector_opcion(&l, "vista", &c->vista, OPCIONES_VISTA, N_OPCIONES_VISTA);
    leer_ajustes_vista(&l, &c->archivos);
    lector_bool(&l, "un_clic", &c->un_clic);
    lector_bool(&l, "confirmar_papelera", &c->confirmar_papelera);
    lector_bool(&l, "confirmar_eliminar", &c->confirmar_eliminar);
    lector_texto(&l, "terminal", &c->terminal);
    lector_bool(&l, "restaurar_sesion", &c->restaurar_sesion);
    lector_bool(&l, "mostrar_usb", &c->mostrar_usb);
    lector_bool(&l, "mostrar_discos_internos", &c->mostrar_discos_internos);
    lector_bool(&l, "sonidos", &c->sonidos);
    lector_numero(&l, "volumen", &c->volumen, 0.0, 1.0);
    lector_bool(&l, "animaciones", &c->animaciones);
    lector_bool(&l, "buscar_actualizaciones", &c->buscar_actualizaciones);
    lector_texto(&l, "repo_actualizaciones", &c->repo_actualizaciones);
    leer_lugares(&l, c);
    leer_atajos(&l, c);
    if (!es_global) avisar_desconocidas(&l);

    lua_seguro_cerrar(L);
    return 0;
}

/* ===================================================================== */
/*  Escritura                                                            */
/* ===================================================================== */

const char *bool_lua(int v) { return v ? "true" : "false"; }

/* Ancho visible de un texto UTF-8 (cuenta caracteres, no bytes). */
static size_t ancho_utf8(const char *t)
{
    size_t n = 0;
    for (; *t; t++)
        if (((unsigned char)*t & 0xC0) != 0x80) n++;
    return n;
}

/* Escribe "  clave = valor," y alinea el comentario en la misma columna. */
static void linea(Cadena *s, const char *codigo, const char *comentario)
{
    cadena_agregar(s, codigo);
    if (comentario && *comentario) {
        size_t ancho = ancho_utf8(codigo);
        for (size_t i = ancho; i < 36; i++) cadena_agregar_n(s, " ", 1);
        cadena_agregar(s, " -- ");
        cadena_agregar(s, comentario);
    }
    cadena_agregar_n(s, "\n", 1);
}

static void campo_texto(Cadena *s, const char *clave, const char *valor, const char *comentario)
{
    Cadena c;
    cadena_iniciar(&c);
    cadena_printf(&c, "  %s = ", clave);
    cadena_agregar_lua(&c, valor ? valor : "");
    cadena_agregar(&c, ",");
    if (!c.sin_memoria) linea(s, c.datos, comentario);
    cadena_liberar(&c);
}

static void campo_bool(Cadena *s, const char *clave, int valor, const char *comentario)
{
    char buf[128];
    snprintf(buf, sizeof buf, "  %s = %s,", clave, bool_lua(valor));
    linea(s, buf, comentario);
}

static void campo_numero(Cadena *s, const char *clave, double valor, const char *comentario)
{
    char buf[128];
    snprintf(buf, sizeof buf, "  %s = %.2f,", clave, valor);
    /* por si el locale usa coma decimal */
    char *coma = strchr(buf + strlen(clave), ',');
    if (coma && coma[1] != '\0') *coma = '.';
    linea(s, buf, comentario);
}

static void campo_entero(Cadena *s, const char *clave, int valor, const char *comentario)
{
    char buf[128];
    snprintf(buf, sizeof buf, "  %s = %d,", clave, valor);
    linea(s, buf, comentario);
}

int solar_config_guardar(const char *archivo, const SolarConfig *c)
{
    static const char *const POS[] = { "arriba", "abajo" };
    static const char *const LADO[] = { "izquierda", "derecha" };

    Cadena s;
    cadena_iniciar(&s);
    cadena_agregar(&s,
        "-- ☀ Solar " SOLAR_VERSION " — configuración\n"
        "--\n"
        "-- Solar reescribe este archivo cada vez que cambias algo en Ajustes,\n"
        "-- pero es Lua normal: puedes editarlo a mano (por ejemplo, para poner\n"
        "-- atajos o lugares que no estén en la ventana de Ajustes).\n"
        "-- Si algo no se entiende, Solar te avisa y usa el valor por defecto.\n"
        "--\n"
        "-- Temas propios: crea archivos .lua en ~/.config/solar/temas/\n"
        "-- (hay un ejemplo comentado en config/temas/ dentro del código de Solar).\n\n"
        "return {\n"
        "  -- ── Apariencia ─────────────────────────────────────────────\n");
    campo_texto(&s, "tema", c->tema ? c->tema : "sol", "id de un tema integrado o de temas/");
    campo_bool(&s, "transparencia", c->transparencia, "fondo transparente (necesita un compositor)");
    campo_numero(&s, "opacidad", c->opacidad, "0.30 (muy transparente) a 1.00 (sólido)");
    campo_numero(&s, "escala", c->escala, "tamaño de toda la interfaz (0.75 a 2.00)");
    campo_texto(&s, "fuente", c->fuente, "\"\" integrada, \"sistema\" o ruta a un .ttf/.otf");
    campo_bool(&s, "barra_titulo_integrada", c->barra_titulo_integrada, "false = bordes de ventana del sistema");

    cadena_agregar(&s, "\n  -- ── Distribución ───────────────────────────────────────────\n");
    campo_texto(&s, "barra_ubicacion", POS[c->barra_ubicacion == SOLAR_POS_ABAJO], "\"arriba\" | \"abajo\"");
    campo_texto(&s, "barra_lateral", LADO[c->barra_lateral == SOLAR_LADO_DERECHA], "\"izquierda\" | \"derecha\"");
    campo_bool(&s, "mostrar_barra_lateral", c->mostrar_barra_lateral, NULL);

    cadena_agregar(&s, "\n  -- ── Archivos ───────────────────────────────────────────────\n");
    int vista = c->vista >= 0 && c->vista <= 3 ? c->vista : 0;
    campo_texto(&s, "vista", NOMBRES_VISTA[vista], "para espacios nuevos: cuadricula | lista | mosaicos | orbita");
    const SolarAjustesVista *a = &c->archivos;
    campo_entero(&s, "tamano_iconos", (int)a->tamano_iconos, "40 a 160");
    campo_bool(&s, "mostrar_ocultos", a->mostrar_ocultos, NULL);
    campo_bool(&s, "carpetas_primero", a->carpetas_primero, NULL);
    int orden = a->orden >= 0 && a->orden <= 3 ? a->orden : 0;
    campo_texto(&s, "orden", NOMBRES_ORDEN[orden], "\"nombre\" | \"tamano\" | \"fecha\" | \"tipo\"");
    campo_bool(&s, "orden_descendente", a->orden_descendente, NULL);
    campo_bool(&s, "un_clic", c->un_clic, "abrir con un solo clic");

    cadena_agregar(&s, "\n  -- ── Vista Órbita ───────────────────────────────────────────\n");
    campo_bool(&s, "orbita_girar", a->orbita_girar, "dan vueltas alrededor de la carpeta");
    campo_numero(&s, "orbita_velocidad", a->orbita_velocidad, "0.10 (lento) a 4.00 (rápido)");
    int oo = a->orbita_orden >= 0 && a->orbita_orden <= 5 ? a->orbita_orden : 0;
    campo_texto(&s, "orbita_orden", NOMBRES_ORBITA[oo], "grande_primero | chico_primero | nombre | tipo | fecha | aleatorio");
    campo_bool(&s, "orbita_electricidad", a->orbita_electricidad, "órbitas eléctricas, siempre encendidas");

    cadena_agregar(&s, "\n  -- ── Comportamiento ─────────────────────────────────────────\n");
    campo_bool(&s, "confirmar_papelera", c->confirmar_papelera, "preguntar antes de mandar a la papelera");
    campo_bool(&s, "confirmar_eliminar", c->confirmar_eliminar, "preguntar antes de eliminar para siempre");
    campo_texto(&s, "terminal", c->terminal, "\"\" = detectar (foot, kitty, alacritty…)");
    campo_bool(&s, "restaurar_sesion", c->restaurar_sesion, "al abrir, volver a las pestañas de la última vez");

    cadena_agregar(&s, "\n  -- ── Dispositivos ───────────────────────────────────────────\n");
    campo_bool(&s, "mostrar_usb", c->mostrar_usb, "memorias USB y tarjetas SD en la barra lateral");
    campo_bool(&s, "mostrar_discos_internos", c->mostrar_discos_internos, "otras particiones (p. ej. la de Windows)");

    cadena_agregar(&s, "\n  -- ── Sonido y animaciones ───────────────────────────────────\n");
    campo_bool(&s, "sonidos", c->sonidos, "efectos al hacer clic, al terminar, errores…");
    campo_numero(&s, "volumen", c->volumen, "0.00 a 1.00");
    campo_bool(&s, "animaciones", c->animaciones, "false = todo instantáneo (para PCs lentas)");

    cadena_agregar(&s, "\n  -- ── Actualizaciones ────────────────────────────────────────\n");
    campo_bool(&s, "buscar_actualizaciones", c->buscar_actualizaciones, "revisar GitHub al abrir (una vez al día)");
    campo_texto(&s, "repo_actualizaciones", c->repo_actualizaciones ? c->repo_actualizaciones : "", "\"usuario/repo\" de GitHub");
    cadena_agregar(&s, "\n");

    cadena_agregar(&s, "  -- ── Lugares (barra lateral) ────────────────────────────────\n"
                       "  -- icono: casa, escritorio, documento, descarga, musica, imagen, video,\n"
                       "  --        mando, codigo, estrella, corazon, nube… (o \"\")\n"
                       "  lugares = {\n");
    for (size_t i = 0; i < c->n_lugares; i++) {
        cadena_agregar(&s, "    { nombre = ");
        cadena_agregar_lua(&s, c->lugares[i].nombre);
        cadena_agregar(&s, ", ruta = ");
        cadena_agregar_lua(&s, c->lugares[i].ruta);
        cadena_agregar(&s, ", icono = ");
        cadena_agregar_lua(&s, c->lugares[i].icono ? c->lugares[i].icono : "");
        cadena_agregar(&s, " },\n");
    }
    cadena_agregar(&s, "  },\n\n"
                       "  -- ── Atajos de teclado ──────────────────────────────────────\n"
                       "  -- Modificadores: Ctrl, Shift, Alt. Teclas: A-Z, 0-9, F1-F12, Left, Right,\n"
                       "  -- Up, Down, Home, End, Enter, Delete, Backspace, Plus, Minus, Comma…\n"
                       "  -- Usa \"\" o false para desactivar un atajo.\n"
                       "  atajos = {\n");
    for (size_t i = 0; i < c->n_atajos; i++) {
        cadena_printf(&s, "    %-18s = ", c->atajos[i].accion);
        cadena_agregar_lua(&s, c->atajos[i].teclas ? c->atajos[i].teclas : "");
        cadena_agregar(&s, ",\n");
    }
    cadena_agregar(&s, "  },\n}\n");

    if (s.sin_memoria) {
        cadena_liberar(&s);
        return ENOMEM;
    }
    char *dir = ruta_padre(archivo);
    if (dir) { crear_dirs(dir, 0755); free(dir); }
    int r = escribir_atomico(archivo, s.datos, s.largo);
    cadena_liberar(&s);
    return r;
}
