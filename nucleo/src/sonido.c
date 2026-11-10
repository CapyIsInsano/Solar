/*
 * ☀ Solar — efectos de sonido
 *
 * Los MP3 se decodifican con minimp3 (CC0, en nucleo/terceros/) y se tocan
 * con PulseAudio/PipeWire (libpulse-simple) o, si no hay, con ALSA. Las dos
 * bibliotecas se abren con dlopen al primer sonido: Solar no las necesita
 * para compilar ni para arrancar, y si no hay ninguna simplemente no suena.
 *
 * Un solo hilo mezclador toca todo: los sonidos se pasan a 48 kHz estéreo
 * al cargarlos, se mezclan en bloques de 20 ms y van a una sola conexión de
 * audio que se abre al primer sonido y se cierra tras 2 s de silencio.
 * Antes cada clic abría su propio hilo y su propia conexión con el
 * servidor de audio: más lento en sonar y más trabajo para la PC. Sin
 * sonidos, el mezclador duerme sin despertarse nunca.
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

#define MAX_SONIDOS  16
#define MAX_VOCES    8      /* sonidos sonando a la vez */
#define FRECUENCIA   48000
#define CANALES      2
#define BLOQUE       960    /* cuadros por bloque: 20 ms */
#define OCIO_MS      2000   /* cerrar la conexión tras este silencio */

typedef struct {
    int16_t *pcm;          /* intercalado */
    size_t   cuadros;      /* muestras por canal */
    int      canales;
    int      frecuencia;
} Sonido;

typedef struct {
    int    id;
    size_t pos;            /* cuadro por el que va */
    float  ganancia;
} Voz;

static Sonido          sonidos[MAX_SONIDOS];
static pthread_mutex_t candado = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t  hay_trabajo = PTHREAD_COND_INITIALIZER;
static Voz             voces[MAX_VOCES];
static int             n_voces = 0;
static int             mezclador_vivo = 0;

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

/* Pasa el sonido a 48 kHz estéreo (lo que espera el mezclador), con
 * remuestreo lineal: para efectos cortos no se nota la diferencia. */
static int a_formato_comun(Sonido *s)
{
    if (s->frecuencia == FRECUENCIA && s->canales == CANALES) return 0;
    size_t n = (size_t)((double)s->cuadros * FRECUENCIA / s->frecuencia);
    if (n == 0) n = 1;
    int16_t *nuevo = malloc(n * CANALES * sizeof(int16_t));
    if (!nuevo) return ENOMEM;
    double paso = (double)s->frecuencia / FRECUENCIA;
    for (size_t i = 0; i < n; i++) {
        double x = (double)i * paso;
        size_t a = (size_t)x;
        if (a >= s->cuadros) a = s->cuadros - 1;
        size_t b = a + 1 < s->cuadros ? a + 1 : a;
        double t = x - (double)a;
        for (int k = 0; k < CANALES; k++) {
            int ka = s->canales == 1 ? 0 : k;
            double va = s->pcm[a * (size_t)s->canales + (size_t)ka];
            double vb = s->pcm[b * (size_t)s->canales + (size_t)ka];
            nuevo[i * CANALES + (size_t)k] = (int16_t)(va + (vb - va) * t);
        }
    }
    free(s->pcm);
    s->pcm = nuevo;
    s->cuadros = n;
    s->canales = CANALES;
    s->frecuencia = FRECUENCIA;
    return 0;
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
    if (s.cuadros > 0 && a_formato_comun(&s) != 0) { free(s.pcm); return ENOMEM; }

    pthread_mutex_lock(&candado);
    Sonido viejo = sonidos[id];
    sonidos[id] = s;
    /* Si se estaba tocando el viejo, se corta (el mezclador lee bajo el candado) */
    for (int i = 0; i < n_voces; i++)
        if (voces[i].id == id) voces[i].pos = (size_t)-1;
    pthread_mutex_unlock(&candado);
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
typedef struct { uint32_t maxlength, tlength, prebuf, minreq, fragsize; } PaAtributos;
typedef void *(*PaNuevoFn)(const char *, const char *, int, const char *, const char *,
                           const PaSpec *, const void *, const PaAtributos *, int *);
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

/* ---------------------------------------------------------------------- */
/*  Mezclador                                                             */
/* ---------------------------------------------------------------------- */

typedef struct {
    void *pa;    /* conexión de PulseAudio/PipeWire */
    void *pcm;   /* o de ALSA */
} Conexion;

static void abrir_conexion(Conexion *c)
{
    if (salida.pulse_ok && !atomic_load(&salida.pulse_fallo)) {
        PaSpec spec = { 3 /* PA_SAMPLE_S16LE */, FRECUENCIA, CANALES };
        /* Búfer de 60 ms: lo justo para no cortarse, sin retrasar el clic */
        uint32_t bytes = FRECUENCIA * CANALES * (uint32_t)sizeof(int16_t) * 60 / 1000;
        PaAtributos at = { (uint32_t)-1, bytes, (uint32_t)-1, (uint32_t)-1, (uint32_t)-1 };
        int err = 0;
        c->pa = salida.pa_nuevo(NULL, "Solar", 1 /* PA_STREAM_PLAYBACK */, NULL, "Efectos de Solar",
                                &spec, NULL, &at, &err);
        if (c->pa) return;
        atomic_store(&salida.pulse_fallo, 1);
    }
    if (salida.alsa_ok) {
        void *pcm = NULL;
        if (salida.alsa_abrir(&pcm, "default", 0 /* PLAYBACK */, 0) < 0 || !pcm) return;
        /* S16_LE = 2, RW_INTERLEAVED = 3, remuestreo por software, 80 ms */
        if (salida.alsa_params(pcm, 2, 3, CANALES, FRECUENCIA, 1, 80000) < 0) {
            salida.alsa_cerrar(pcm);
            return;
        }
        c->pcm = pcm;
    }
}

static void cerrar_conexion(Conexion *c)
{
    int err = 0;
    if (c->pa) {
        salida.pa_vaciar(c->pa, &err);
        salida.pa_liberar(c->pa);
    }
    if (c->pcm) {
        salida.alsa_vaciar(c->pcm);
        salida.alsa_cerrar(c->pcm);
    }
    c->pa = c->pcm = NULL;
}

static void escribir(Conexion *c, const int16_t *bloque, size_t cuadros)
{
    if (c->pa) {
        int err = 0;
        if (salida.pa_escribir(c->pa, bloque, cuadros * CANALES * sizeof(int16_t), &err) < 0) {
            /* El servidor se cayó: pasamos a ALSA para lo que sigue */
            salida.pa_liberar(c->pa);
            c->pa = NULL;
            atomic_store(&salida.pulse_fallo, 1);
        }
        return;
    }
    size_t hecho = 0;
    while (c->pcm && hecho < cuadros) {
        long r = salida.alsa_escribir(c->pcm, bloque + hecho * CANALES, cuadros - hecho);
        if (r < 0) {
            if (salida.alsa_recuperar(c->pcm, (int)r, 1) < 0) {
                salida.alsa_cerrar(c->pcm);
                c->pcm = NULL;
            }
            continue;
        }
        hecho += (size_t)r;
    }
}

/* Mezcla hasta un bloque de las voces activas (con el candado tomado).
 * Devuelve cuántos cuadros salieron (0 si ya no queda nada). */
static size_t mezclar(int16_t *bloque)
{
    int32_t acum[BLOQUE * CANALES];
    memset(acum, 0, sizeof acum);
    size_t usados = 0;
    for (int i = 0; i < n_voces; i++) {
        Voz *v = &voces[i];
        const Sonido *s = &sonidos[v->id];
        if (v->pos >= s->cuadros) {
            voces[i--] = voces[--n_voces];
            continue;
        }
        size_t k = s->cuadros - v->pos;
        if (k > BLOQUE) k = BLOQUE;
        const int16_t *p = s->pcm + v->pos * CANALES;
        for (size_t j = 0; j < k * CANALES; j++) acum[j] += (int32_t)((float)p[j] * v->ganancia);
        v->pos += k;
        if (k > usados) usados = k;
    }
    /* Varios sonidos juntos pueden pasarse del máximo: en vez de cortar en
     * seco (se oye feo), se comprime suave lo que pasa de ~75 % */
    const int32_t rodilla = 24576, resto = 32767 - rodilla;
    for (size_t j = 0; j < usados * CANALES; j++) {
        int32_t x = acum[j], ax = x < 0 ? -x : x;
        if (ax > rodilla) {
            int32_t exceso = ax - rodilla;
            ax = rodilla + (int32_t)((int64_t)exceso * resto / (exceso + resto));
            x = x < 0 ? -ax : ax;
        }
        bloque[j] = (int16_t)x;
    }
    return usados;
}

static void *hilo_mezclador(void *arg)
{
    (void)arg;
    Conexion c = { 0 };
    int16_t bloque[BLOQUE * CANALES];
    pthread_mutex_lock(&candado);
    for (;;) {
        if (n_voces == 0) {
            if (!c.pa && !c.pcm) {
                /* Nada que tocar y nada abierto: dormir hasta el próximo sonido */
                pthread_cond_wait(&hay_trabajo, &candado);
                continue;
            }
            struct timespec hasta;
            clock_gettime(CLOCK_REALTIME, &hasta);
            hasta.tv_sec += OCIO_MS / 1000;
            int r = pthread_cond_timedwait(&hay_trabajo, &candado, &hasta);
            if (r == ETIMEDOUT && n_voces == 0) {
                pthread_mutex_unlock(&candado);
                cerrar_conexion(&c);
                pthread_mutex_lock(&candado);
            }
            continue;
        }
        size_t cuadros = mezclar(bloque);
        pthread_mutex_unlock(&candado);
        if (cuadros > 0) {
            if (!c.pa && !c.pcm) abrir_conexion(&c);
            escribir(&c, bloque, cuadros);   /* bloquea al ritmo del audio */
        }
        pthread_mutex_lock(&candado);
    }
    return NULL;
}

int solar_sonido_reproducir(int32_t id, float volumen)
{
    if (id < 0 || id >= MAX_SONIDOS) return EINVAL;
    if (volumen <= 0.001f) return 0;
    if (volumen > 1.0f) volumen = 1.0f;
    pthread_once(&salida.una_vez, cargar_salidas);
    if (!salida.pulse_ok && !salida.alsa_ok) return ENOTSUP;

    pthread_mutex_lock(&candado);
    if (sonidos[id].cuadros == 0) {
        pthread_mutex_unlock(&candado);
        return ENOENT;
    }
    if (!mezclador_vivo) {
        pthread_t h;
        pthread_attr_t attr;
        pthread_attr_init(&attr);
        pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
        pthread_attr_setstacksize(&attr, 256 * 1024);
        int r = pthread_create(&h, &attr, hilo_mezclador, NULL);
        pthread_attr_destroy(&attr);
        if (r) {
            pthread_mutex_unlock(&candado);
            return r;
        }
        mezclador_vivo = 1;
    }
    /* Muchos a la vez (clics muy rápidos): el más viejo deja su lugar */
    if (n_voces == MAX_VOCES) {
        memmove(voces, voces + 1, (MAX_VOCES - 1) * sizeof *voces);
        n_voces--;
    }
    /* Curva de volumen perceptual (cuadrática) */
    voces[n_voces++] = (Voz){ id, 0, volumen * volumen };
    pthread_cond_signal(&hay_trabajo);
    pthread_mutex_unlock(&candado);
    return 0;
}
