-- ☀ Solar — ejemplo de configuración (todas las opciones explicadas)
--
-- Solar crea ~/.config/solar/config.lua la primera vez que lo abres, y lo
-- reescribe cada vez que cambias algo en Ajustes. Este archivo es solo de
-- referencia: copia lo que quieras a tu config.lua.
--
-- Es Lua de verdad (5.4), en un entorno seguro: puedes usar variables,
-- cuentas, if, os.getenv(), os.date()… pero no io, os.execute ni require.
-- Si algo no se entiende, Solar te dice qué fue y usa el valor por defecto.
--
-- OJO: como Ajustes reescribe el archivo, la lógica que pongas (if,
-- variables) se convierte en valores fijos la próxima vez que cambies algo
-- desde la ventana. Los temas (temas/*.lua) nunca se reescriben.

local en_la_laptop = os.getenv("HOSTNAME") == "x220"

return {
  -- ── Apariencia ──────────────────────────────────────────────────────
  tema = "sol",              -- sol, aurora (clara), eclipse, nebulosa, venus,
                             -- neptuno, marte, mercurio, luna (clara)… o uno
                             -- tuyo
  transparencia = false,     -- fondo transparente (necesita compositor:
                             -- Niri, Hyprland, Sway con blur, KWin, picom…)
  opacidad = 0.85,           -- 0.30 (muy transparente) a 1.00 (sólido)
  escala = en_la_laptop and 1.25 or 1.0,  -- tamaño de toda la interfaz
  fuente = "",               -- "" integrada, "sistema" (fontconfig) o la
                             -- ruta a un .ttf/.otf, p. ej. "~/.fonts/Inter.ttf"
  barra_titulo_integrada = true,  -- Solar dibuja sus botones de ventana;
                                  -- false = bordes del sistema (al reiniciar)

  -- ── Sonido y animaciones ────────────────────────────────────────────
  sonidos = true,            -- clics, avisos, errores… (los de Capyro)
  volumen = 0.6,             -- 0.0 a 1.0
  animaciones = true,        -- transiciones; apágalas si tu PC es lenta

  -- ── Actualizaciones ─────────────────────────────────────────────────
  buscar_actualizaciones = true,           -- revisar GitHub al abrir (1 vez al día)
  repo_actualizaciones = "CapyIsInsano/Solar",

  -- ── Distribución ────────────────────────────────────────────────────
  barra_ubicacion = "arriba",     -- "arriba" | "abajo"
  barra_lateral = "izquierda",    -- "izquierda" | "derecha"
  mostrar_barra_lateral = true,

  -- ── Archivos ────────────────────────────────────────────────────────
  -- (un espacio de trabajo puede tener los suyos: ver espacios.lua)
  vista = "cuadricula",      -- la de las pestañas nuevas de los espacios
                             -- nuevos: "cuadricula" | "lista" | "mosaicos"
                             -- | "orbita" (cada pestaña recuerda la suya)
  tamano_iconos = 72,        -- 40 a 160 (también Ctrl + rueda del mouse)
  mostrar_ocultos = false,   -- archivos que empiezan con punto
  carpetas_primero = true,
  orden = "nombre",          -- "nombre" | "tamano" | "fecha" | "tipo"
  orden_descendente = false,
  un_clic = false,           -- abrir con un solo clic (como KDE)
  restaurar_sesion = true,   -- al abrir Solar, volver a las pestañas de la
                             -- última vez (cada espacio recuerda las suyas)

  -- ── Vista Órbita ────────────────────────────────────────────────────
  -- La carpeta abierta va en el centro (como el sol) y lo de adentro le da
  -- vueltas, todo con su ícono de siempre.
  orbita_girar = true,       -- que giren (se detienen solos al pasar el
                             -- mouse por encima)
  orbita_velocidad = 1.0,    -- 0.1 a 4.0 (1.0 = una vuelta en 48 s)
  orbita_orden = "grande_primero",  -- del centro hacia afuera: "grande_primero"
                             -- | "chico_primero" | "nombre" | "tipo"
                             -- | "fecha" (recientes cerca) | "aleatorio"
  orbita_electricidad = true,  -- órbitas eléctricas, siempre encendidas
                               -- (false = líneas tranquilas)

  -- ── Comportamiento ──────────────────────────────────────────────────
  confirmar_papelera = false,
  confirmar_eliminar = true,
  terminal = "",             -- "" detecta (foot, kitty, alacritty, wezterm…)
                             -- o pon un comando: "kitty --single-instance"

  -- ── Dispositivos ────────────────────────────────────────────────────
  mostrar_usb = true,              -- memorias USB y tarjetas SD
  mostrar_discos_internos = false, -- otras particiones (p. ej. Windows)

  -- ── Lugares (barra lateral) ─────────────────────────────────────────
  -- icono = nombre de un logotipo: casa, escritorio, documento, descarga,
  -- musica, imagen, video, mando, codigo, terminal, estrella, corazon,
  -- nube, candado, trabajo, escuela, libro, arte, chip, cohete, sol, luna…
  -- (la lista completa está en Ajustes → Lugares, clic en el ícono)
  lugares = {
    { nombre = "Carpeta personal", ruta = "~",            icono = "casa" },
    { nombre = "Descargas",        ruta = "~/Descargas",  icono = "descarga" },
    { nombre = "Juegos",           ruta = "~/Juegos",     icono = "mando" },
    { nombre = "Proyectos",        ruta = "~/Proyectos",  icono = "codigo" },
    "~/Música",   -- también vale solo la ruta
  },

  -- ── Atajos de teclado ───────────────────────────────────────────────
  -- Modificadores: Ctrl, Shift, Alt. Teclas: A-Z, 0-9, F1-F12, Left,
  -- Right, Up, Down, Home, End, Enter, Delete, Backspace, Plus, Minus,
  -- Comma… Usa "" o false para quitar un atajo. Los que no pongas quedan
  -- como vienen.
  -- Si un atajo tuyo choca con uno que trae una versión nueva de Solar,
  -- gana el tuyo y el nuevo se queda sin atajo (Solar te avisa).
  atajos = {
    nueva_carpeta = "Ctrl+Shift+N",
    personalizar  = "Ctrl+E",       -- color y logotipo de la carpeta
    terminal      = "F4",
    vista_orbita  = "Ctrl+4",
    nueva_pestana = "Ctrl+T",
    espacios      = "Ctrl+Shift+E", -- menú de espacios de trabajo
    eliminar      = false,          -- sin atajo para eliminar para siempre
  },
}
