/*
 * ☀ Solar — efectos de sonido
 *
 * Los MP3 se decodifican con minimp3 (CC0, en nucleo/terceros/) y se tocan
 * con PulseAudio/PipeWire (libpulse-simple) o, si no hay, con ALSA. Las dos
 * bibliotecas se abren con dlopen al primer sonido: Solar no las necesita
 * para compilar ni para arrancar, y si no hay ninguna simplemente no suena.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <dlfcn.h>
#include <pthread.h>
#include <stdatomic.h>

#define MINIMP3_IMPLEMENTATION
#define MINIMP3_NO_SIMD_WARNINGS
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wsign-compare"
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wconversion"
#endif
#include "../terceros/minimp3/minimp3.h"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

#define MAX_SONIDOS      16
#define MAX_SIMULTANEOS  6

typedef struct {
    int16_t *pcm;          /* intercalado */
    size_t   cuadros;      /* muestras por canal */
    int      canales;
    int      frecuencia;
} Sonido;

static Sonido          sonidos[MAX_SONIDOS];
static pthread_mutex_t candado = PTHREAD_MUTEX_INITIALIZER;
static atomic_int      sonando = 0;

/* ---------------------------------------------------------------------- */
/*  Decodificar                                                           */
/* ---------------------------------------------------------------------- */

/* Quita el silencio del principio y del final (los archivos suelen traer
 * un segundo de nada al final) y suaviza las orillas para que no truenen. */
static void recortar(Sonido *s)
{
    const int umbral = 90;          /* ≈ -51 dB */
    size_t total = s->cuadros, ini = 0, fin = total;
    int c = s->canales;
    while (ini < total) {
        int alto = 0;
        for (int k = 0; k < c; k++) if (abs(s->pcm[ini * c + k]) > umbral) alto = 1;
        if (alto) break;
        ini++;
    }
    while (fin > ini) {
        int alto = 0;
        for (int k = 0; k < c; k++) if (abs(s->pcm[(fin - 1) * c + k]) > umbral) alto = 1;
        if (alto) break;
        fin--;
    }
    if (fin <= ini) { s->cuadros = 0; return; }
    size_t margen_ini = (size_t)s->frecuencia * 4 / 1000;   /* 4 ms */
    size_t margen_fin = (size_t)s->frecuencia * 30 / 1000;  /* 30 ms */
    ini = ini > margen_ini ? ini - margen_ini : 0;
    fin = fin + margen_fin < total ? fin + margen_fin : total;
    size_t n = fin - ini;
    memmove(s->pcm, s->pcm + ini * c, n * c * sizeof(int16_t));
    s->cuadros = n;

    /* Desvanecer los últimos 20 ms */
    size_t f = (size_t)s->frecuencia / 50;
    if (f > n) f = n;
    for (size_t i = 0; i < f; i++) {
        double g = (double)i / (double)f;
        for (int k = 0; k < c; k++) {
            int16_t *m = &s->pcm[(n - 1 - i) * c + k];
            *m = (int16_t)(*m * g);
        }
    }
}

int solar_sonido_cargar(int32_t id, const uint8_t *datos, size_t tam)
{
    if (id < 0 || id >= MAX_SONIDOS || !datos || tam == 0) return EINVAL;

    mp3dec_t *dec = malloc(sizeof *dec);
    if (!dec) return ENOMEM;
    mp3dec_init(dec);

    Sonido s = { 0 };
    size_t capacidad = 0;
    mp3d_sample_t cuadro[MINIMP3_MAX_SAMPLES_PER_FRAME];
    size_t pos = 0;
    while (pos < tam) {
        mp3dec_frame_info_t info;
        int restante = tam - pos > INT32_MAX ? INT32_MAX : (int)(tam - pos);
        int muestras = mp3dec_decode_frame(dec, datos + pos, restante, cuadro, &info);
        if (info.frame_bytes == 0) break;          /* no hay más cuadros */
        pos += (size_t)info.frame_bytes;
        if (muestras <= 0) continue;               /* basura o metadatos */
        if (s.canales == 0) { s.canales = info.channels; s.frecuencia = info.hz; }
        if (info.channels != s.canales || info.hz != s.frecuencia) continue;
        size_t nuevas = (size_t)muestras;
        if (s.cuadros + nuevas > capacidad) {
            size_t cap = capacidad ? capacidad * 2 : 44100;
            while (cap < s.cuadros + nuevas) cap *= 2;
            int16_t *p = realloc(s.pcm, cap * (size_t)s.canales * sizeof(int16_t));
            if (!p) { free(s.pcm); free(dec); return ENOMEM; }
            s.pcm = p;
            capacidad = cap;
        }
        memcpy(s.pcm + s.cuadros * (size_t)s.canales, cuadro, nuevas * (size_t)s.canales * sizeof(int16_t));
        s.cuadros += nuevas;
    }
    free(dec);
    if (s.cuadros == 0 || s.canales < 1 || s.canales > 2) { free(s.pcm); return EINVAL; }
    recortar(&s);

    pthread_mutex_lock(&candado);
    Sonido viejo = sonidos[id];
    sonidos[id] = s;
    pthread_mutex_unlock(&candado);
    /* Si alguien estaba tocando el viejo, tiene su propia copia */
    free(viejo.pcm);
    return 0;
}

double solar_sonido_duracion(int32_t id)
{
    if (id < 0 || id >= MAX_SONIDOS) return 0;
    pthread_mutex_lock(&candado);
    double d = sonidos[id].frecuencia ? (double)sonidos[id].cuadros / sonidos[id].frecuencia : 0;
    pthread_mutex_unlock(&candado);
    return d;
}

/* ---------------------------------------------------------------------- */
/*  Salidas de audio (dlopen)                                             */
/* ---------------------------------------------------------------------- */

/* PulseAudio simple (también lo ofrece PipeWire con pipewire-pulse) */
typedef struct { int formato; uint32_t frecuencia; uint8_t canales; } PaSpec;
typedef void *(*PaNuevoFn)(const char *, const char *, int, const char *, const char *,
                           const PaSpec *, const void *, const void *, int *);
typedef int (*PaEscribirFn)(void *, const void *, size_t, int *);
typedef int (*PaVaciarFn)(void *, int *);
typedef void (*PaLiberarFn)(void *);

/* ALSA */
typedef int (*AlsaAbrirFn)(void **, const char *, int, int);
typedef int (*AlsaParamsFn)(void *, int, int, unsigned, unsigned, int, unsigned);
typedef long (*AlsaEscribirFn)(void *, const void *, unsigned long);
typedef int (*AlsaRecuperarFn)(void *, int, int);
typedef int (*AlsaVaciarFn)(void *);
typedef int (*AlsaCerrarFn)(void *);

static struct {
    pthread_once_t una_vez;
    int            pulse_ok;
    PaNuevoFn      pa_nuevo;
    PaEscribirFn   pa_escribir;
    PaVaciarFn     pa_vaciar;
    PaLiberarFn    pa_liberar;
    int            alsa_ok;
    AlsaAbrirFn    alsa_abrir;
    AlsaParamsFn   alsa_params;
    AlsaEscribirFn alsa_escribir;
    AlsaRecuperarFn alsa_recuperar;
    AlsaVaciarFn   alsa_vaciar;
    AlsaCerrarFn   alsa_cerrar;
    atomic_int     pulse_fallo;    /* si el servidor no contesta, usar ALSA */
} salida = { .una_vez = PTHREAD_ONCE_INIT };

/* ALSA escribe sus quejas en stderr ("cannot find card 0"…); las callamos */
static void alsa_callado(const char *archivo, int linea, const char *funcion, int err, const char *formato, ...)
{
    (void)archivo; (void)linea; (void)funcion; (void)err; (void)formato;
}

static void cargar_salidas(void)
{
    void *pa = dlopen("libpulse-simple.so.0", RTLD_NOW | RTLD_LOCAL);
    if (pa) {
        salida.pa_nuevo    = (PaNuevoFn)dlsym(pa, "pa_simple_new");
        salida.pa_escribir = (PaEscribirFn)dlsym(pa, "pa_simple_write");
        salida.pa_vaciar   = (PaVaciarFn)dlsym(pa, "pa_simple_drain");
        salida.pa_liberar  = (PaLiberarFn)dlsym(pa, "pa_simple_free");
        salida.pulse_ok = salida.pa_nuevo && salida.pa_escribir && salida.pa_vaciar && salida.pa_liberar;
    }
    void *as = dlopen("libasound.so.2", RTLD_NOW | RTLD_LOCAL);
    if (as) {
        salida.alsa_abrir     = (AlsaAbrirFn)dlsym(as, "snd_pcm_open");
        salida.alsa_params    = (AlsaParamsFn)dlsym(as, "snd_pcm_set_params");
        salida.alsa_escribir  = (AlsaEscribirFn)dlsym(as, "snd_pcm_writei");
        salida.alsa_recuperar = (AlsaRecuperarFn)dlsym(as, "snd_pcm_recover");
        salida.alsa_vaciar    = (AlsaVaciarFn)dlsym(as, "snd_pcm_drain");
        salida.alsa_cerrar    = (AlsaCerrarFn)dlsym(as, "snd_pcm_close");
        salida.alsa_ok = salida.alsa_abrir && salida.alsa_params && salida.alsa_escribir &&
                         salida.alsa_recuperar && salida.alsa_vaciar && salida.alsa_cerrar;
        typedef int (*ManejadorFn)(void (*)(const char *, int, const char *, int, const char *, ...));
        ManejadorFn poner = (ManejadorFn)dlsym(as, "snd_lib_error_set_handler");
        if (poner) poner(alsa_callado);
    }
}

const char *solar_sonido_salida(void)
{
    pthread_once(&salida.una_vez, cargar_salidas);
    if (salida.pulse_ok && !atomic_load(&salida.pulse_fallo)) return "PulseAudio / PipeWire";
    if (salida.alsa_ok) return "ALSA";
    return "";
}

static int tocar_pulse(const Sonido *s)
{
    PaSpec spec = { 3 /* PA_SAMPLE_S16LE */, (uint32_t)s->frecuencia, (uint8_t)s->canales };
    int err = 0;
    void *pa = salida.pa_nuevo(NULL, "Solar", 1 /* PA_STREAM_PLAYBACK */, NULL, "Efecto de Solar",
                               &spec, NULL, NULL, &err);
    if (!pa) return -1;
    salida.pa_escribir(pa, s->pcm, s->cuadros * (size_t)s->canales * sizeof(int16_t), &err);
    salida.pa_vaciar(pa, &err);
    salida.pa_liberar(pa);
    return 0;
}

static int tocar_alsa(const Sonido *s)
{
    void *pcm = NULL;
    if (salida.alsa_abrir(&pcm, "default", 0 /* PLAYBACK */, 0) < 0 || !pcm) return -1;
    /* S16_LE = 2, RW_INTERLEAVED = 3, remuestreo por software, 80 ms */
    if (salida.alsa_params(pcm, 2, 3, (unsigned)s->canales, (unsigned)s->frecuencia, 1, 80000) < 0) {
        salida.alsa_cerrar(pcm);
        return -1;
    }
    size_t hecho = 0;
    while (hecho < s->cuadros) {
        long r = salida.alsa_escribir(pcm, s->pcm + hecho * (size_t)s->canales, s->cuadros - hecho);
        if (r < 0) {
            if (salida.alsa_recuperar(pcm, (int)r, 1) < 0) break;
            continue;
        }
        hecho += (size_t)r;
    }
    salida.alsa_vaciar(pcm);
    salida.alsa_cerrar(pcm);
    return 0;
}

static void *hilo_tocar(void *arg)
{
    Sonido *s = arg;
    int ok = -1;
    if (salida.pulse_ok && !atomic_load(&salida.pulse_fallo)) {
        ok = tocar_pulse(s);
        if (ok != 0) atomic_store(&salida.pulse_fallo, 1);
    }
    if (ok != 0 && salida.alsa_ok) tocar_alsa(s);
    free(s->pcm);
    free(s);
    atomic_fetch_sub(&sonando, 1);
    return NULL;
}

int solar_sonido_reproducir(int32_t id, float volumen)
{
    if (id < 0 || id >= MAX_SONIDOS) return EINVAL;
    if (volumen <= 0.001f) return 0;
    if (volumen > 1.0f) volumen = 1.0f;
    pthread_once(&salida.una_vez, cargar_salidas);
    if (!salida.pulse_ok && !salida.alsa_ok) return ENOTSUP;

    /* Demasiados a la vez (clics muy rápidos): ese se omite */
    if (atomic_fetch_add(&sonando, 1) >= MAX_SIMULTANEOS) {
        atomic_fetch_sub(&sonando, 1);
        return EBUSY;
    }

    Sonido *copia = calloc(1, sizeof *copia);
    if (!copia) { atomic_fetch_sub(&sonando, 1); return ENOMEM; }
    pthread_mutex_lock(&candado);
    const Sonido *s = &sonidos[id];
    if (s->cuadros > 0) {
        size_t n = s->cuadros * (size_t)s->canales;
        copia->pcm = malloc(n * sizeof(int16_t));
        if (copia->pcm) {
            /* Curva de volumen perceptual (cuadrática) */
            float g = volumen * volumen;
            for (size_t i = 0; i < n; i++) copia->pcm[i] = (int16_t)(s->pcm[i] * g);
            copia->cuadros = s->cuadros;
            copia->canales = s->canales;
            copia->frecuencia = s->frecuencia;
        }
    }
    pthread_mutex_unlock(&candado);
    if (!copia->pcm) {
        free(copia);
        atomic_fetch_sub(&sonando, 1);
        return ENOENT;
    }

    pthread_t h;
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
    pthread_attr_setstacksize(&attr, 256 * 1024);
    int r = pthread_create(&h, &attr, hilo_tocar, copia);
    pthread_attr_destroy(&attr);
    if (r) {
        free(copia->pcm);
        free(copia);
        atomic_fetch_sub(&sonando, 1);
        return r;
    }
    return 0;
}
