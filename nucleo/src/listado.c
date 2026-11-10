/*
 * ☀ Solar — listado de carpetas, orden natural, tipos de archivo y propiedades
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <dirent.h>
#include <fcntl.h>
#include <grp.h>
#include <pthread.h>
#include <pwd.h>
#include <strings.h>
#include <time.h>
#include <unistd.h>

/* ===================================================================== */
/*  Tipos de archivo por extensión                                       */
/* ===================================================================== */

typedef struct {
    const char *ext;
    unsigned char cat;
    const char *descripcion;
} TipoConocido;

static TipoConocido TIPOS[] = {
    /* Imágenes */
    {"png", SOLAR_CAT_IMAGEN, "Imagen PNG"},
    {"jpg", SOLAR_CAT_IMAGEN, "Imagen JPEG"},
    {"jpeg", SOLAR_CAT_IMAGEN, "Imagen JPEG"},
    {"jfif", SOLAR_CAT_IMAGEN, "Imagen JPEG"},
    {"gif", SOLAR_CAT_IMAGEN, "Imagen GIF"},
    {"webp", SOLAR_CAT_IMAGEN, "Imagen WebP"},
    {"bmp", SOLAR_CAT_IMAGEN, "Imagen BMP"},
    {"svg", SOLAR_CAT_IMAGEN, "Imagen SVG"},
    {"svgz", SOLAR_CAT_IMAGEN, "Imagen SVG comprimida"},
    {"ico", SOLAR_CAT_IMAGEN, "Ícono"},
    {"tif", SOLAR_CAT_IMAGEN, "Imagen TIFF"},
    {"tiff", SOLAR_CAT_IMAGEN, "Imagen TIFF"},
    {"heic", SOLAR_CAT_IMAGEN, "Imagen HEIC"},
    {"heif", SOLAR_CAT_IMAGEN, "Imagen HEIF"},
    {"avif", SOLAR_CAT_IMAGEN, "Imagen AVIF"},
    {"jxl", SOLAR_CAT_IMAGEN, "Imagen JPEG XL"},
    {"qoi", SOLAR_CAT_IMAGEN, "Imagen QOI"},
    {"xcf", SOLAR_CAT_IMAGEN, "Imagen de GIMP"},
    {"psd", SOLAR_CAT_IMAGEN, "Documento de Photoshop"},
    {"kra", SOLAR_CAT_IMAGEN, "Dibujo de Krita"},
    {"ora", SOLAR_CAT_IMAGEN, "Imagen OpenRaster"},
    {"cr2", SOLAR_CAT_IMAGEN, "Fotografía RAW"},
    {"nef", SOLAR_CAT_IMAGEN, "Fotografía RAW"},
    {"arw", SOLAR_CAT_IMAGEN, "Fotografía RAW"},
    {"dng", SOLAR_CAT_IMAGEN, "Fotografía RAW"},
    {"raw", SOLAR_CAT_IMAGEN, "Fotografía RAW"},
    /* Audio */
    {"mp3", SOLAR_CAT_AUDIO, "Audio MP3"},
    {"flac", SOLAR_CAT_AUDIO, "Audio FLAC"},
    {"ogg", SOLAR_CAT_AUDIO, "Audio Ogg"},
    {"oga", SOLAR_CAT_AUDIO, "Audio Ogg"},
    {"opus", SOLAR_CAT_AUDIO, "Audio Opus"},
    {"wav", SOLAR_CAT_AUDIO, "Audio WAV"},
    {"m4a", SOLAR_CAT_AUDIO, "Audio AAC"},
    {"aac", SOLAR_CAT_AUDIO, "Audio AAC"},
    {"wma", SOLAR_CAT_AUDIO, "Audio WMA"},
    {"aiff", SOLAR_CAT_AUDIO, "Audio AIFF"},
    {"mid", SOLAR_CAT_AUDIO, "Secuencia MIDI"},
    {"midi", SOLAR_CAT_AUDIO, "Secuencia MIDI"},
    {"mka", SOLAR_CAT_AUDIO, "Audio Matroska"},
    /* Video */
    {"mp4", SOLAR_CAT_VIDEO, "Video MP4"},
    {"m4v", SOLAR_CAT_VIDEO, "Video MP4"},
    {"mkv", SOLAR_CAT_VIDEO, "Video Matroska"},
    {"webm", SOLAR_CAT_VIDEO, "Video WebM"},
    {"avi", SOLAR_CAT_VIDEO, "Video AVI"},
    {"mov", SOLAR_CAT_VIDEO, "Video QuickTime"},
    {"wmv", SOLAR_CAT_VIDEO, "Video WMV"},
    {"flv", SOLAR_CAT_VIDEO, "Video Flash"},
    {"mpg", SOLAR_CAT_VIDEO, "Video MPEG"},
    {"mpeg", SOLAR_CAT_VIDEO, "Video MPEG"},
    {"3gp", SOLAR_CAT_VIDEO, "Video 3GP"},
    {"ogv", SOLAR_CAT_VIDEO, "Video Ogg"},
    /* Comprimidos */
    {"zip", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido ZIP"},
    {"rar", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido RAR"},
    {"7z", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido 7-Zip"},
    {"tar", SOLAR_CAT_COMPRIMIDO, "Archivo TAR"},
    {"gz", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido gzip"},
    {"tgz", SOLAR_CAT_COMPRIMIDO, "Archivo TAR comprimido (gzip)"},
    {"bz2", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido bzip2"},
    {"xz", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido XZ"},
    {"txz", SOLAR_CAT_COMPRIMIDO, "Archivo TAR comprimido (XZ)"},
    {"zst", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido Zstandard"},
    {"lz4", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido LZ4"},
    {"lzma", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido LZMA"},
    {"lz", SOLAR_CAT_COMPRIMIDO, "Archivo comprimido lzip"},
    {"cab", SOLAR_CAT_COMPRIMIDO, "Archivo CAB"},
    /* PDF y documentos */
    {"pdf", SOLAR_CAT_PDF, "Documento PDF"},
    {"doc", SOLAR_CAT_DOCUMENTO, "Documento de Word"},
    {"docx", SOLAR_CAT_DOCUMENTO, "Documento de Word"},
    {"odt", SOLAR_CAT_DOCUMENTO, "Documento de texto ODF"},
    {"rtf", SOLAR_CAT_DOCUMENTO, "Texto enriquecido"},
    {"epub", SOLAR_CAT_DOCUMENTO, "Libro electrónico EPUB"},
    {"mobi", SOLAR_CAT_DOCUMENTO, "Libro electrónico MOBI"},
    {"djvu", SOLAR_CAT_DOCUMENTO, "Documento DjVu"},
    {"xls", SOLAR_CAT_HOJA_CALCULO, "Hoja de cálculo de Excel"},
    {"xlsx", SOLAR_CAT_HOJA_CALCULO, "Hoja de cálculo de Excel"},
    {"ods", SOLAR_CAT_HOJA_CALCULO, "Hoja de cálculo ODF"},
    {"csv", SOLAR_CAT_HOJA_CALCULO, "Valores separados por comas"},
    {"tsv", SOLAR_CAT_HOJA_CALCULO, "Valores separados por tabuladores"},
    {"ppt", SOLAR_CAT_PRESENTACION, "Presentación de PowerPoint"},
    {"pptx", SOLAR_CAT_PRESENTACION, "Presentación de PowerPoint"},
    {"odp", SOLAR_CAT_PRESENTACION, "Presentación ODF"},
    /* Texto */
    {"txt", SOLAR_CAT_TEXTO, "Documento de texto"},
    {"md", SOLAR_CAT_TEXTO, "Documento Markdown"},
    {"markdown", SOLAR_CAT_TEXTO, "Documento Markdown"},
    {"rst", SOLAR_CAT_TEXTO, "Documento reStructuredText"},
    {"log", SOLAR_CAT_TEXTO, "Registro"},
    {"nfo", SOLAR_CAT_TEXTO, "Información"},
    {"ini", SOLAR_CAT_TEXTO, "Archivo de configuración"},
    {"conf", SOLAR_CAT_TEXTO, "Archivo de configuración"},
    {"cfg", SOLAR_CAT_TEXTO, "Archivo de configuración"},
    {"desktop", SOLAR_CAT_TEXTO, "Lanzador de aplicación"},
    {"srt", SOLAR_CAT_TEXTO, "Subtítulos SRT"},
    {"ass", SOLAR_CAT_TEXTO, "Subtítulos ASS"},
    {"vtt", SOLAR_CAT_TEXTO, "Subtítulos WebVTT"},
    /* Código */
    {"c", SOLAR_CAT_CODIGO, "Código fuente C"},
    {"h", SOLAR_CAT_CODIGO, "Cabecera C"},
    {"cpp", SOLAR_CAT_CODIGO, "Código fuente C++"},
    {"cc", SOLAR_CAT_CODIGO, "Código fuente C++"},
    {"cxx", SOLAR_CAT_CODIGO, "Código fuente C++"},
    {"hpp", SOLAR_CAT_CODIGO, "Cabecera C++"},
    {"hh", SOLAR_CAT_CODIGO, "Cabecera C++"},
    {"rs", SOLAR_CAT_CODIGO, "Código fuente Rust"},
    {"lua", SOLAR_CAT_CODIGO, "Script de Lua"},
    {"py", SOLAR_CAT_CODIGO, "Script de Python"},
    {"pyw", SOLAR_CAT_CODIGO, "Script de Python"},
    {"js", SOLAR_CAT_CODIGO, "Código JavaScript"},
    {"mjs", SOLAR_CAT_CODIGO, "Código JavaScript"},
    {"ts", SOLAR_CAT_CODIGO, "Código TypeScript"},
    {"jsx", SOLAR_CAT_CODIGO, "Código JSX"},
    {"tsx", SOLAR_CAT_CODIGO, "Código TSX"},
    {"java", SOLAR_CAT_CODIGO, "Código fuente Java"},
    {"kt", SOLAR_CAT_CODIGO, "Código fuente Kotlin"},
    {"go", SOLAR_CAT_CODIGO, "Código fuente Go"},
    {"rb", SOLAR_CAT_CODIGO, "Script de Ruby"},
    {"php", SOLAR_CAT_CODIGO, "Script de PHP"},
    {"cs", SOLAR_CAT_CODIGO, "Código fuente C#"},
    {"swift", SOLAR_CAT_CODIGO, "Código fuente Swift"},
    {"zig", SOLAR_CAT_CODIGO, "Código fuente Zig"},
    {"nim", SOLAR_CAT_CODIGO, "Código fuente Nim"},
    {"dart", SOLAR_CAT_CODIGO, "Código fuente Dart"},
    {"hs", SOLAR_CAT_CODIGO, "Código fuente Haskell"},
    {"ml", SOLAR_CAT_CODIGO, "Código fuente OCaml"},
    {"ex", SOLAR_CAT_CODIGO, "Código fuente Elixir"},
    {"exs", SOLAR_CAT_CODIGO, "Script de Elixir"},
    {"erl", SOLAR_CAT_CODIGO, "Código fuente Erlang"},
    {"clj", SOLAR_CAT_CODIGO, "Código fuente Clojure"},
    {"scala", SOLAR_CAT_CODIGO, "Código fuente Scala"},
    {"pl", SOLAR_CAT_CODIGO, "Script de Perl"},
    {"r", SOLAR_CAT_CODIGO, "Script de R"},
    {"jl", SOLAR_CAT_CODIGO, "Código fuente Julia"},
    {"sh", SOLAR_CAT_CODIGO, "Script de shell"},
    {"bash", SOLAR_CAT_CODIGO, "Script de Bash"},
    {"zsh", SOLAR_CAT_CODIGO, "Script de Zsh"},
    {"fish", SOLAR_CAT_CODIGO, "Script de Fish"},
    {"ps1", SOLAR_CAT_CODIGO, "Script de PowerShell"},
    {"bat", SOLAR_CAT_CODIGO, "Script de Windows"},
    {"html", SOLAR_CAT_CODIGO, "Página web HTML"},
    {"htm", SOLAR_CAT_CODIGO, "Página web HTML"},
    {"css", SOLAR_CAT_CODIGO, "Hoja de estilos CSS"},
    {"scss", SOLAR_CAT_CODIGO, "Hoja de estilos SCSS"},
    {"json", SOLAR_CAT_CODIGO, "Datos JSON"},
    {"toml", SOLAR_CAT_CODIGO, "Configuración TOML"},
    {"yaml", SOLAR_CAT_CODIGO, "Datos YAML"},
    {"yml", SOLAR_CAT_CODIGO, "Datos YAML"},
    {"xml", SOLAR_CAT_CODIGO, "Documento XML"},
    {"sql", SOLAR_CAT_CODIGO, "Consulta SQL"},
    {"asm", SOLAR_CAT_CODIGO, "Código ensamblador"},
    {"s", SOLAR_CAT_CODIGO, "Código ensamblador"},
    {"ino", SOLAR_CAT_CODIGO, "Sketch de Arduino"},
    {"kicad_pcb", SOLAR_CAT_CODIGO, "Placa de KiCad"},
    {"kicad_sch", SOLAR_CAT_CODIGO, "Esquemático de KiCad"},
    {"kicad_pro", SOLAR_CAT_CODIGO, "Proyecto de KiCad"},
    {"cmake", SOLAR_CAT_CODIGO, "Script de CMake"},
    {"mk", SOLAR_CAT_CODIGO, "Makefile"},
    {"gd", SOLAR_CAT_CODIGO, "Script de Godot"},
    {"glsl", SOLAR_CAT_CODIGO, "Shader GLSL"},
    {"vert", SOLAR_CAT_CODIGO, "Shader de vértices"},
    {"frag", SOLAR_CAT_CODIGO, "Shader de fragmentos"},
    {"vim", SOLAR_CAT_CODIGO, "Script de Vim"},
    {"el", SOLAR_CAT_CODIGO, "Código Emacs Lisp"},
    {"diff", SOLAR_CAT_CODIGO, "Diferencias (diff)"},
    {"patch", SOLAR_CAT_CODIGO, "Parche"},
    {"ebuild", SOLAR_CAT_CODIGO, "Ebuild de Gentoo"},
    {"eclass", SOLAR_CAT_CODIGO, "Eclass de Gentoo"},
    {"nix", SOLAR_CAT_CODIGO, "Expresión de Nix"},
    {"tex", SOLAR_CAT_CODIGO, "Documento LaTeX"},
    {"ipynb", SOLAR_CAT_CODIGO, "Cuaderno de Jupyter"},
    /* Ejecutables */
    {"appimage", SOLAR_CAT_EJECUTABLE, "Aplicación AppImage"},
    {"exe", SOLAR_CAT_EJECUTABLE, "Programa de Windows"},
    {"msi", SOLAR_CAT_EJECUTABLE, "Instalador de Windows"},
    {"run", SOLAR_CAT_EJECUTABLE, "Instalador ejecutable"},
    {"bin", SOLAR_CAT_EJECUTABLE, "Archivo binario"},
    {"elf", SOLAR_CAT_EJECUTABLE, "Ejecutable ELF"},
    {"so", SOLAR_CAT_EJECUTABLE, "Biblioteca compartida"},
    {"dll", SOLAR_CAT_EJECUTABLE, "Biblioteca de Windows"},
    {"o", SOLAR_CAT_EJECUTABLE, "Código objeto"},
    {"a", SOLAR_CAT_EJECUTABLE, "Biblioteca estática"},
    {"jar", SOLAR_CAT_EJECUTABLE, "Aplicación Java"},
    /* Fuentes */
    {"ttf", SOLAR_CAT_FUENTE, "Fuente TrueType"},
    {"otf", SOLAR_CAT_FUENTE, "Fuente OpenType"},
    {"ttc", SOLAR_CAT_FUENTE, "Colección de fuentes"},
    {"woff", SOLAR_CAT_FUENTE, "Fuente web"},
    {"woff2", SOLAR_CAT_FUENTE, "Fuente web"},
    {"pcf", SOLAR_CAT_FUENTE, "Fuente de mapa de bits"},
    {"bdf", SOLAR_CAT_FUENTE, "Fuente de mapa de bits"},
    /* Discos */
    {"iso", SOLAR_CAT_DISCO, "Imagen de disco ISO"},
    {"img", SOLAR_CAT_DISCO, "Imagen de disco"},
    {"qcow2", SOLAR_CAT_DISCO, "Disco virtual QEMU"},
    {"vdi", SOLAR_CAT_DISCO, "Disco virtual VirtualBox"},
    {"vmdk", SOLAR_CAT_DISCO, "Disco virtual VMware"},
    {"vhd", SOLAR_CAT_DISCO, "Disco virtual"},
    {"vhdx", SOLAR_CAT_DISCO, "Disco virtual"},
    {"dmg", SOLAR_CAT_DISCO, "Imagen de disco de macOS"},
    /* Paquetes */
    {"deb", SOLAR_CAT_PAQUETE, "Paquete Debian"},
    {"rpm", SOLAR_CAT_PAQUETE, "Paquete RPM"},
    {"apk", SOLAR_CAT_PAQUETE, "Paquete APK"},
    {"flatpak", SOLAR_CAT_PAQUETE, "Paquete Flatpak"},
    {"flatpakref", SOLAR_CAT_PAQUETE, "Referencia de Flatpak"},
    {"snap", SOLAR_CAT_PAQUETE, "Paquete Snap"},
    {"whl", SOLAR_CAT_PAQUETE, "Paquete de Python"},
    {"gem", SOLAR_CAT_PAQUETE, "Paquete de Ruby"},
    {"crate", SOLAR_CAT_PAQUETE, "Paquete de Rust"},
    {"xpi", SOLAR_CAT_PAQUETE, "Extensión de Firefox"},
    {"torrent", SOLAR_CAT_GENERICO, "Torrent"},
    {"bak", SOLAR_CAT_GENERICO, "Copia de seguridad"},
};

typedef struct {
    const char *nombre;
    unsigned char cat;
    const char *descripcion;
} NombreConocido;

static const NombreConocido NOMBRES[] = {
    {"Makefile", SOLAR_CAT_CODIGO, "Makefile"},
    {"makefile", SOLAR_CAT_CODIGO, "Makefile"},
    {"GNUmakefile", SOLAR_CAT_CODIGO, "Makefile"},
    {"CMakeLists.txt", SOLAR_CAT_CODIGO, "Proyecto de CMake"},
    {"Dockerfile", SOLAR_CAT_CODIGO, "Dockerfile"},
    {"PKGBUILD", SOLAR_CAT_CODIGO, "Receta de paquete de Arch"},
    {"APKBUILD", SOLAR_CAT_CODIGO, "Receta de paquete de Alpine"},
    {"Cargo.toml", SOLAR_CAT_CODIGO, "Proyecto de Rust"},
    {"Cargo.lock", SOLAR_CAT_CODIGO, "Dependencias de Rust"},
    {"meson.build", SOLAR_CAT_CODIGO, "Proyecto de Meson"},
    {"README", SOLAR_CAT_TEXTO, "Léeme"},
    {"LICENSE", SOLAR_CAT_TEXTO, "Licencia"},
    {"LICENCE", SOLAR_CAT_TEXTO, "Licencia"},
    {"COPYING", SOLAR_CAT_TEXTO, "Licencia"},
    {"AUTHORS", SOLAR_CAT_TEXTO, "Autores"},
    {"CHANGELOG", SOLAR_CAT_TEXTO, "Registro de cambios"},
    {".bashrc", SOLAR_CAT_CODIGO, "Configuración de Bash"},
    {".zshrc", SOLAR_CAT_CODIGO, "Configuración de Zsh"},
    {".profile", SOLAR_CAT_CODIGO, "Perfil de shell"},
    {".gitignore", SOLAR_CAT_TEXTO, "Reglas de Git"},
    {".xinitrc", SOLAR_CAT_CODIGO, "Inicio de X11"},
};

static pthread_once_t tabla_ordenada = PTHREAD_ONCE_INIT;

static int comparar_tipo(const void *a, const void *b)
{
    return strcmp(((const TipoConocido *)a)->ext, ((const TipoConocido *)b)->ext);
}

static void ordenar_tabla(void) { qsort(TIPOS, SOLAR_LEN(TIPOS), sizeof TIPOS[0], comparar_tipo); }

/* Extensión en minúsculas ("tar.gz" cuenta como "gz", salvo casos raros). */
static const char *extension(const char *nombre, char *buf, size_t tam)
{
    const char *punto = strrchr(nombre, '.');
    if (!punto || punto == nombre || punto[1] == '\0') return NULL;
    if (strlen(punto + 1) >= tam) return NULL;
    strcpy(buf, punto + 1);
    a_minusculas(buf);
    return buf;
}

static const TipoConocido *buscar_tipo(const char *nombre)
{
    pthread_once(&tabla_ordenada, ordenar_tabla);
    char ext[24];
    if (!extension(nombre, ext, sizeof ext)) return NULL;
    TipoConocido clave = { ext, 0, NULL };
    return bsearch(&clave, TIPOS, SOLAR_LEN(TIPOS), sizeof TIPOS[0], comparar_tipo);
}

static const NombreConocido *buscar_nombre(const char *nombre)
{
    int es_leeme = !strcasecmp(nombre, "README") || !strcasecmp(nombre, "README.txt");
    for (size_t i = 0; i < SOLAR_LEN(NOMBRES); i++) {
        if (!strcmp(NOMBRES[i].nombre, nombre)) return &NOMBRES[i];
        if (es_leeme && !strcmp(NOMBRES[i].nombre, "README")) return &NOMBRES[i];
    }
    return NULL;
}

int solar_categoria(const char *nombre, uint32_t modo, int tipo)
{
    if (tipo == SOLAR_TIPO_CARPETA) return SOLAR_CAT_CARPETA;
    if (tipo != SOLAR_TIPO_ARCHIVO) return SOLAR_CAT_GENERICO;
    const NombreConocido *n = buscar_nombre(nombre);
    if (n) return n->cat;
    /* Paquetes de Arch: algo.pkg.tar.zst */
    if (strstr(nombre, ".pkg.tar.")) return SOLAR_CAT_PAQUETE;
    const TipoConocido *t = buscar_tipo(nombre);
    if (t) return t->cat;
    if (modo & 0111) return SOLAR_CAT_EJECUTABLE;
    return SOLAR_CAT_GENERICO;
}

void solar_describir_tipo(const char *nombre, int categoria, int tipo, int es_enlace,
                          char *salida, size_t tam)
{
    if (!salida || tam == 0) return;
    if (tipo == SOLAR_TIPO_CARPETA) {
        copiar_texto(salida, tam, es_enlace ? "Enlace a carpeta" : "Carpeta");
        return;
    }
    if (tipo == SOLAR_TIPO_ENLACE_ROTO) { copiar_texto(salida, tam, "Enlace roto"); return; }
    if (tipo == SOLAR_TIPO_ESPECIAL) { copiar_texto(salida, tam, "Archivo especial"); return; }

    const NombreConocido *n = buscar_nombre(nombre);
    if (n) { copiar_texto(salida, tam, n->descripcion); return; }
    if (strstr(nombre, ".pkg.tar.")) { copiar_texto(salida, tam, "Paquete de Arch"); return; }
    if (termina_con(nombre, ".tar.gz") || termina_con(nombre, ".tar.xz") ||
        termina_con(nombre, ".tar.bz2") || termina_con(nombre, ".tar.zst")) {
        snprintf(salida, tam, "Archivo TAR comprimido (%s)", strrchr(nombre, '.') + 1);
        return;
    }
    const TipoConocido *t = buscar_tipo(nombre);
    if (t) { copiar_texto(salida, tam, t->descripcion); return; }
    if (categoria == SOLAR_CAT_EJECUTABLE) { copiar_texto(salida, tam, "Programa"); return; }
    char ext[24];
    if (extension(nombre, ext, sizeof ext)) {
        for (char *p = ext; *p; p++)
            if (*p >= 'a' && *p <= 'z') *p = (char)(*p - 32);
        snprintf(salida, tam, "Archivo %s", ext);
        return;
    }
    copiar_texto(salida, tam, "Archivo");
}

/* ===================================================================== */
/*  Comparación natural                                                  */
/* ===================================================================== */

/* Devuelve un valor de orden para un código Unicode: letras sin acento y
 * en minúscula (multiplicado por 2), con la ñ justo después de la n. */
static uint32_t plegar(uint32_t cp)
{
    if (cp >= 'A' && cp <= 'Z') cp += 32;
    if (cp < 0x80) return cp * 2;
    switch (cp) {
    case 0xD1: case 0xF1: return 'n' * 2 + 1;   /* Ñ ñ */
    case 0xDF: return 's' * 2;                   /* ß */
    case 0xC6: case 0xE6: return 'a' * 2;        /* Æ æ */
    case 0x152: case 0x153: return 'o' * 2;      /* Œ œ */
    default: break;
    }
    if ((cp >= 0xC0 && cp <= 0xC5) || (cp >= 0xE0 && cp <= 0xE5) || (cp >= 0x100 && cp <= 0x105)) return 'a' * 2;
    if (cp == 0xC7 || cp == 0xE7 || (cp >= 0x106 && cp <= 0x10D)) return 'c' * 2;
    if (cp >= 0x10E && cp <= 0x111) return 'd' * 2;
    if ((cp >= 0xC8 && cp <= 0xCB) || (cp >= 0xE8 && cp <= 0xEB) || (cp >= 0x112 && cp <= 0x11B)) return 'e' * 2;
    if (cp >= 0x11C && cp <= 0x123) return 'g' * 2;
    if (cp >= 0x124 && cp <= 0x127) return 'h' * 2;
    if ((cp >= 0xCC && cp <= 0xCF) || (cp >= 0xEC && cp <= 0xEF) || (cp >= 0x128 && cp <= 0x131)) return 'i' * 2;
    if (cp >= 0x134 && cp <= 0x135) return 'j' * 2;
    if (cp >= 0x136 && cp <= 0x138) return 'k' * 2;
    if (cp >= 0x139 && cp <= 0x142) return 'l' * 2;
    if (cp >= 0x143 && cp <= 0x14B) return 'n' * 2;
    if ((cp >= 0xD2 && cp <= 0xD6) || cp == 0xD8 || (cp >= 0xF2 && cp <= 0xF6) || cp == 0xF8 ||
        (cp >= 0x14C && cp <= 0x151)) return 'o' * 2;
    if (cp >= 0x154 && cp <= 0x159) return 'r' * 2;
    if (cp >= 0x15A && cp <= 0x161) return 's' * 2;
    if (cp >= 0x162 && cp <= 0x167) return 't' * 2;
    if ((cp >= 0xD9 && cp <= 0xDC) || (cp >= 0xF9 && cp <= 0xFC) || (cp >= 0x168 && cp <= 0x173)) return 'u' * 2;
    if (cp >= 0x174 && cp <= 0x175) return 'w' * 2;
    if (cp == 0xDD || cp == 0xFD || cp == 0xFF || (cp >= 0x176 && cp <= 0x178)) return 'y' * 2;
    if (cp >= 0x179 && cp <= 0x17E) return 'z' * 2;
    /* Griego y cirílico: mayúsculas -> minúsculas */
    if (cp >= 0x391 && cp <= 0x3A9) cp += 0x20;
    else if (cp >= 0x410 && cp <= 0x42F) cp += 0x20;
    else if (cp >= 0x400 && cp <= 0x40F) cp += 0x50;
    return cp * 2 + 0x1000;
}

static uint32_t siguiente_letra(const unsigned char **p)
{
    const unsigned char *s = *p;
    uint32_t c = s[0];
    if (c < 0x80) { *p = s + 1; return plegar(c); }
    uint32_t cp;
    int largo;
    if ((c & 0xE0) == 0xC0 && (s[1] & 0xC0) == 0x80) {
        cp = ((c & 0x1F) << 6) | (s[1] & 0x3F);
        largo = 2;
    } else if ((c & 0xF0) == 0xE0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80) {
        cp = ((c & 0x0F) << 12) | ((uint32_t)(s[1] & 0x3F) << 6) | (s[2] & 0x3F);
        largo = 3;
    } else if ((c & 0xF8) == 0xF0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80 && (s[3] & 0xC0) == 0x80) {
        cp = ((c & 0x07) << 18) | ((uint32_t)(s[1] & 0x3F) << 12) | ((uint32_t)(s[2] & 0x3F) << 6) | (s[3] & 0x3F);
        largo = 4;
    } else {
        *p = s + 1;
        return 0x200000 + c; /* byte inválido: al final */
    }
    *p = s + largo;
    return plegar(cp);
}

static int es_digito(unsigned char c) { return c >= '0' && c <= '9'; }

int solar_comparar_natural(const char *a, const char *b)
{
    const unsigned char *pa = (const unsigned char *)a, *pb = (const unsigned char *)b;
    while (*pa && *pb) {
        /* Camino rápido: letras y signos ASCII (lo más común). Da lo mismo
         * que plegar(), sin decodificar UTF-8 ni llamar funciones. */
        unsigned char x = *pa, y = *pb;
        if (x < 0x80 && y < 0x80 && !es_digito(x) && !es_digito(y)) {
            if (x != y) {
                if (x >= 'A' && x <= 'Z') x += 32;
                if (y >= 'A' && y <= 'Z') y += 32;
                if (x != y) return x < y ? -1 : 1;
            }
            pa++;
            pb++;
            continue;
        }
        if (es_digito(*pa) && es_digito(*pb)) {
            const unsigned char *ia = pa, *ib = pb;
            while (*ia == '0' && es_digito(ia[1])) ia++;
            while (*ib == '0' && es_digito(ib[1])) ib++;
            const unsigned char *fa = ia, *fb = ib;
            while (es_digito(*fa)) fa++;
            while (es_digito(*fb)) fb++;
            size_t la = (size_t)(fa - ia), lb = (size_t)(fb - ib);
            if (la != lb) return la < lb ? -1 : 1;
            int r = memcmp(ia, ib, la);
            if (r) return r < 0 ? -1 : 1;
            pa = fa;
            pb = fb;
            continue;
        }
        uint32_t ca = siguiente_letra(&pa), cb = siguiente_letra(&pb);
        if (ca != cb) return ca < cb ? -1 : 1;
    }
    if (*pa) return 1;
    if (*pb) return -1;
    return 0;
}

/* ===================================================================== */
/*  Listado                                                              */
/* ===================================================================== */

/* Nombres listados en el archivo ".hidden" de la carpeta (estilo GTK). */
static char *leer_ocultos(int dir_fd)
{
    enum { MAXIMO = 64 * 1024 };
    int fd = openat(dir_fd, ".hidden", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return NULL;
    /* Queda como "\nnombre1\nnombre2\n\0" para buscar "\nnombre\n" */
    char *buf = malloc(MAXIMO + 3);
    if (!buf) { close(fd); return NULL; }
    size_t n = 0;
    while (n < MAXIMO) {
        ssize_t r = read(fd, buf + 1 + n, MAXIMO - n);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) break;
        n += (size_t)r;
    }
    close(fd);
    if (n == 0) { free(buf); return NULL; }
    buf[0] = '\n';
    buf[n + 1] = '\n';
    buf[n + 2] = '\0';
    for (size_t i = 0; i < n + 2; i++)
        if (buf[i] == '\r') buf[i] = '\n';
    return buf;
}

static int esta_en_ocultos(const char *ocultos, const char *nombre)
{
    if (!ocultos) return 0;
    size_t n = strlen(nombre);
    for (const char *p = strchr(ocultos, '\n'); p; p = strchr(p + 1, '\n')) {
        if (strncmp(p + 1, nombre, n) == 0 && (p[n + 1] == '\n' || p[n + 1] == '\0')) return 1;
    }
    return 0;
}

int solar_listar(const char *ruta, SolarListado *salida)
{
    salida->entradas = NULL;
    salida->cantidad = 0;
    DIR *d = opendir(ruta);
    if (!d) return errno;
    int fd = dirfd(d);
    char *ocultos = leer_ocultos(fd);

    size_t cap = 64, n = 0;
    SolarEntrada *v = malloc(cap * sizeof *v);
    if (!v) { free(ocultos); closedir(d); return ENOMEM; }

    struct dirent *de;
    for (;;) {
        errno = 0;
        de = readdir(d);
        if (!de) break;
        const char *nombre = de->d_name;
        if (!strcmp(nombre, ".") || !strcmp(nombre, "..")) continue;
        struct stat st;
        if (fstatat(fd, nombre, &st, AT_SYMLINK_NOFOLLOW) != 0) continue;

        SolarEntrada e;
        memset(&e, 0, sizeof e);
        e.es_enlace = S_ISLNK(st.st_mode) ? 1 : 0;
        int roto = 0;
        if (e.es_enlace) {
            struct stat destino;
            if (fstatat(fd, nombre, &destino, 0) == 0) st = destino;
            else roto = 1;
        }
        if (roto) e.tipo = SOLAR_TIPO_ENLACE_ROTO;
        else if (S_ISDIR(st.st_mode)) e.tipo = SOLAR_TIPO_CARPETA;
        else if (S_ISREG(st.st_mode)) e.tipo = SOLAR_TIPO_ARCHIVO;
        else e.tipo = SOLAR_TIPO_ESPECIAL;

        e.tamano = e.tipo == SOLAR_TIPO_ARCHIVO ? (uint64_t)st.st_size : 0;
        e.modificado = (int64_t)st.st_mtime;
        e.modo = (uint32_t)st.st_mode;
        size_t largo = strlen(nombre);
        e.es_oculto = (nombre[0] == '.' || (largo > 1 && nombre[largo - 1] == '~') ||
                       esta_en_ocultos(ocultos, nombre)) ? 1 : 0;
        e.categoria = (uint8_t)solar_categoria(nombre, e.modo, e.tipo);
        e.nombre = solar_strdup(nombre);
        if (!e.nombre) break;

        if (n == cap) {
            cap *= 2;
            SolarEntrada *q = realloc(v, cap * sizeof *q);
            if (!q) { free(e.nombre); break; }
            v = q;
        }
        v[n++] = e;
    }
    int error_lectura = errno;
    free(ocultos);
    closedir(d);
    salida->entradas = v;
    salida->cantidad = n;
    /* Un error a mitad de carpeta (p. ej. USB desconectada) se informa, pero
     * devolvemos lo que se alcanzó a leer. */
    return de == NULL && error_lectura != 0 && n == 0 ? error_lectura : 0;
}

void solar_listado_liberar(SolarListado *listado)
{
    if (!listado) return;
    for (size_t i = 0; i < listado->cantidad; i++) free(listado->entradas[i].nombre);
    free(listado->entradas);
    listado->entradas = NULL;
    listado->cantidad = 0;
}

static _Thread_local struct {
    int orden;
    int descendente;
    int carpetas_primero;
} criterio;

static int comparar_numeros(int64_t a, int64_t b) { return (a > b) - (a < b); }

static int comparar_entradas(const void *pa, const void *pb)
{
    const SolarEntrada *a = pa, *b = pb;
    if (criterio.carpetas_primero) {
        int ca = a->tipo == SOLAR_TIPO_CARPETA, cb = b->tipo == SOLAR_TIPO_CARPETA;
        if (ca != cb) return cb - ca; /* carpetas primero aunque sea descendente */
    }
    int r = 0;
    switch (criterio.orden) {
    case SOLAR_ORDEN_TAMANO:
        r = a->tamano > b->tamano ? 1 : (a->tamano < b->tamano ? -1 : 0);
        break;
    case SOLAR_ORDEN_FECHA:
        r = comparar_numeros(a->modificado, b->modificado);
        break;
    case SOLAR_ORDEN_TIPO: {
        r = (int)a->categoria - (int)b->categoria;
        if (r == 0) {
            const char *ea = strrchr(a->nombre, '.'), *eb = strrchr(b->nombre, '.');
            r = solar_comparar_natural(ea && ea != a->nombre ? ea : "", eb && eb != b->nombre ? eb : "");
        }
        break;
    }
    default:
        break;
    }
    if (r == 0) r = solar_comparar_natural(a->nombre, b->nombre);
    if (r == 0) r = strcmp(a->nombre, b->nombre);
    return criterio.descendente ? -r : r;
}

void solar_ordenar(SolarListado *listado, int orden, int descendente, int carpetas_primero)
{
    if (!listado || listado->cantidad < 2) return;
    criterio.orden = orden;
    criterio.descendente = descendente;
    criterio.carpetas_primero = carpetas_primero;
    qsort(listado->entradas, listado->cantidad, sizeof listado->entradas[0], comparar_entradas);
}

/* ===================================================================== */
/*  Información y permisos                                               */
/* ===================================================================== */

int solar_info(const char *ruta, SolarInfo *info)
{
    memset(info, 0, sizeof *info);
    struct stat st;
    if (lstat(ruta, &st) != 0) return errno;
    int roto = 0;
    if (S_ISLNK(st.st_mode)) {
        info->es_enlace = 1;
        ssize_t n = readlink(ruta, info->destino_enlace, sizeof info->destino_enlace - 1);
        if (n > 0) info->destino_enlace[n] = '\0';
        struct stat destino;
        if (stat(ruta, &destino) == 0) st = destino;
        else roto = 1;
    }
    if (roto) info->tipo = SOLAR_TIPO_ENLACE_ROTO;
    else if (S_ISDIR(st.st_mode)) info->tipo = SOLAR_TIPO_CARPETA;
    else if (S_ISREG(st.st_mode)) info->tipo = SOLAR_TIPO_ARCHIVO;
    else info->tipo = SOLAR_TIPO_ESPECIAL;

    info->tamano = (uint64_t)st.st_size;
    info->modificado = (int64_t)st.st_mtime;
    info->accedido = (int64_t)st.st_atime;
    info->modo = (uint32_t)st.st_mode;
    info->uid = (uint32_t)st.st_uid;
    info->gid = (uint32_t)st.st_gid;
    info->categoria = (uint8_t)solar_categoria(ruta_base(ruta), info->modo, info->tipo);

#ifdef STATX_BTIME
    struct statx sx;
    if (statx(AT_FDCWD, ruta, roto ? AT_SYMLINK_NOFOLLOW : 0, STATX_BTIME, &sx) == 0 &&
        (sx.stx_mask & STATX_BTIME))
        info->creado = (int64_t)sx.stx_btime.tv_sec;
#endif

    char buf[4096];
    struct passwd pw, *pres = NULL;
    if (getpwuid_r(st.st_uid, &pw, buf, sizeof buf, &pres) == 0 && pres)
        copiar_texto(info->propietario, sizeof info->propietario, pres->pw_name);
    else
        snprintf(info->propietario, sizeof info->propietario, "%u", (unsigned)st.st_uid);
    struct group gr, *gres = NULL;
    if (getgrgid_r(st.st_gid, &gr, buf, sizeof buf, &gres) == 0 && gres)
        copiar_texto(info->grupo, sizeof info->grupo, gres->gr_name);
    else
        snprintf(info->grupo, sizeof info->grupo, "%u", (unsigned)st.st_gid);
    return 0;
}

int solar_cambiar_permisos(const char *ruta, uint32_t modo)
{
    return chmod(ruta, (mode_t)(modo & 07777)) == 0 ? 0 : errno;
}

typedef struct {
    uint64_t bytes, archivos, carpetas, vistos;
    dev_t dispositivo;
    SolarProgresoFn progreso;
    void *datos;
    int cancelado;
} Conteo;

static void contar_total(int fd, Conteo *c, int profundidad)
{
    DIR *d = fdopendir(fd);
    if (!d) { close(fd); return; }
    struct dirent *de;
    while (!c->cancelado && (de = readdir(d))) {
        if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
        struct stat st;
        if (fstatat(dirfd(d), de->d_name, &st, AT_SYMLINK_NOFOLLOW) != 0) continue;
        c->vistos++;
        if (S_ISDIR(st.st_mode)) {
            c->carpetas++;
            if (st.st_dev == c->dispositivo && profundidad < 4096) {
                int hijo = openat(dirfd(d), de->d_name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                if (hijo >= 0) contar_total(hijo, c, profundidad + 1);
            }
        } else {
            c->archivos++;
            if (S_ISREG(st.st_mode)) c->bytes += (uint64_t)st.st_size;
        }
        if (c->progreso && (c->vistos % 256) == 0 &&
            c->progreso(c->datos, c->bytes, 0, c->vistos, 0, de->d_name))
            c->cancelado = 1;
    }
    closedir(d);
}

int solar_tamano_total(const char *ruta, uint64_t *bytes, uint64_t *archivos,
                       uint64_t *carpetas, SolarProgresoFn progreso, void *datos)
{
    Conteo c;
    memset(&c, 0, sizeof c);
    c.progreso = progreso;
    c.datos = datos;
    struct stat st;
    if (stat(ruta, &st) != 0) return errno;
    if (!S_ISDIR(st.st_mode)) {
        *bytes = (uint64_t)st.st_size;
        *archivos = 1;
        *carpetas = 0;
        return 0;
    }
    c.dispositivo = st.st_dev;
    int fd = open(ruta, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (fd < 0) return errno;
    contar_total(fd, &c, 0);
    *bytes = c.bytes;
    *archivos = c.archivos;
    *carpetas = c.carpetas;
    if (c.cancelado) return ECANCELED;
    if (progreso) progreso(datos, c.bytes, c.bytes, c.vistos, c.vistos, "");
    return 0;
}
