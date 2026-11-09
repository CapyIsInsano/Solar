# Registro de cambios

## v0.1.0-alpha — 2026-11-08

Primera versión de Solar.

### Carpetas personalizadas
- Color de carpeta: 21 colores listos, selector de color y código hex.
- Logotipo de carpeta: 54 logotipos (Phosphor) o un texto/emoji propio, con
  su propio color (o automático según la carpeta).
- Todas las carpetas se dibujan en vectorial con la forma del logo de Solar.
- El estilo sigue a la carpeta al renombrarla o moverla desde Solar, viaja
  con las copias y se olvida al eliminarla para siempre.

### Apariencia y Ajustes
- 9 temas integrados (Sol, Aurora, Eclipse, Nebulosa, Venus, Neptuno,
  Marte, Mercurio y Luna) y temas propios en Lua con herencia (`base = "…"`);
  editor de temas en Ajustes que guarda archivos `.lua`.
- Fondo transparente con opacidad ajustable, escala de la interfaz y
  fuente integrada, del sistema o desde un archivo.
- Barra de ubicación arriba o abajo; barra lateral a la izquierda, a la
  derecha o escondida; barra de título propia con botones de ventana.
- Lugares editables (nombre, ruta, ícono y orden).
- Atajos de teclado configurables con captura de teclas y aviso de choques.

### Archivos
- Vistas de cuadrícula y lista, orden natural sin importar acentos (la ñ va
  después de la n), carpetas primero, ocultos (punto, `~` y `.hidden`).
- Selección con clic, Ctrl, Shift, banda y teclado; búsqueda al escribir.
- Copiar, cortar, pegar, duplicar y mover con progreso, cancelación y
  conflictos (conservar ambos, reemplazar/combinar, omitir).
- Papelera freedesktop (compatible con otros gestores), también en memorias
  USB; restaurar, eliminar para siempre y vaciar.
- Arrastrar y soltar a carpetas, lugares, memorias USB y la barra de ubicación.
- Propiedades con tamaño de carpetas, fechas y permisos editables.
- Abrir, abrir con…, abrir terminal aquí y abrir en ventana nueva.
- Memorias USB y tarjetas SD con espacio libre, montar y expulsar (udisks2).
- La vista se actualiza sola (inotify) y avisa si se desconecta la unidad.

### Sonido, actualizaciones y escritorio
- **Efectos de sonido** (los de Capyro): inicio, clic, efecto, listo,
  retracción y error. Se decodifican con minimp3 y suenan por
  PulseAudio/PipeWire o ALSA (cargados con dlopen: si no hay ninguno, Solar
  simplemente no suena). Interruptor, volumen y botón «Probar» en Ajustes.
- **Actualizaciones desde GitHub**: *Ajustes → Actualizaciones* busca
  releases (o etiquetas) más nuevas en el repo, descarga el binario si la
  versión lo trae o compila el código con cargo, y reemplaza el programa sin
  desinstalar (con pkexec si está en una carpeta del sistema). Revisión
  automática una vez al día, que se puede apagar.
- **Borrar caché** (`~/.cache/solar`: descargas y compilaciones).
- **Abrir con… detecta tus programas**: tipos MIME con shared-mime-info,
  lanzadores `.desktop` (también Flatpak) y `mimeapps.list`. Submenú en el
  clic derecho con íconos, diálogo con búsqueda y «Usar siempre este
  programa». Los programas de terminal (Neovim, Vim…) se abren en tu
  terminal.
- **Logo en la barra de tareas**: ícono con poco margen (se ve grande en
  docks y barras), ícono de la ventana (X11) y, si
  Solar no está instalado, un lanzador e íconos en `~/.local/share` para que
  Wayland (Niri, Hyprland, KDE, GNOME) lo encuentre.
- **Animaciones**: entrada de los archivos al abrir una carpeta, íconos que
  se levantan al pasar el mouse, ventanas y diálogos que aparecen suavemente,
  avisos que se deslizan y cambio de tema con transición. Se pueden apagar.
- Queda claro en todos lados: **Solar por ahora es exclusivo de Linux**.

### Núcleo en C
- API en `nucleo/include/solar.h`, Lua 5.4 embebido en un entorno seguro
  (sin io/os.execute/require, con límite de memoria e instrucciones).
- 218 revisiones de pruebas con AddressSanitizer y UBSan.
