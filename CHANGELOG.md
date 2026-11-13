# Registro de cambios

## v0.1.2-alpha «Órbita» — 2026-11-12

Vistas, Órbita, pestañas y espacios de trabajo.

### Una base para todas las vistas
- Las vistas comparten una sola interfaz: cada una solo dice dónde va cada
  archivo, cómo se dibuja y a dónde llevan las flechas. La selección, la
  banda, arrastrar y soltar, el clic derecho, el teclado, las animaciones y
  la navegación son los mismos para todas.
- **4 vistas, como en Windows**: íconos (muy grandes, grandes, medianos y
  pequeños), lista (detalles), **mosaicos** (nueva: el ícono con el tipo y el
  tamaño al lado) y **Órbita** (nueva). Atajos Ctrl+1 a Ctrl+4.
- Menú **Vista** en la barra, como el de Windows: tamaños de íconos, vistas
  y mostrar ocultos.
- Cambiar de vista no pierde nada: la selección, el archivo con el foco (la
  vista nueva lo busca) y la carpeta siguen igual. Cada pestaña recuerda su
  vista.

### Órbita
- La carpeta abierta va en el **centro**, como el sol, y lo que tiene
  adentro le da vueltas en anillos. Todo se ve con su ícono de siempre: la
  carpeta del centro y las que giran con su color y logotipo (la carpeta
  personal con su casita), y los archivos con el de su tipo. Los más
  pesados se ven un poco más grandes.
- **Giran** como planetas de verdad (los de afuera más despacio), con
  velocidad ajustable, y se detienen solos al pasar el mouse, al arrastrar o
  con un menú abierto.
- **Acomodo** del centro hacia afuera: más grandes, más chicos, por nombre, por
  tipo, más recientes o al azar (con botón para revolver). Al cambiarlo, todo
  se desliza a su lugar nuevo.
- **Electricidad**: cada órbita es un rayo continuo, siempre encendido, que
  tiembla suave sin parpadear. Se apaga en la barrita de la órbita o en
  *Ajustes → Archivos*, y se adelgaza si los anillos quedan muy juntos.
- La rueda acerca hacia el puntero, el botón de la rueda mueve la vista y la
  mira la centra. ← y → recorren la órbita; ↑ y ↓ saltan de anillo. Doble
  clic en la carpeta del centro: subir.
- Al abrir una carpeta, lo de adentro sale disparado del centro.
- Todo lo demás funciona igual que en las otras vistas: banda, soltar
  archivos en una carpeta que gira, clic derecho, renombrar…
- Se dibuja a unos 30 cuadros por segundo (20 con más de 1 500 archivos) y
  solo mientras algo se mueve.

### Pestañas
- Barra de pestañas como la de Windows 11, con los botones de la ventana a
  la derecha.
- Ctrl+T nueva, Ctrl+W cerrar, Ctrl+Tab / Ctrl+Shift+Tab cambiar y
  Ctrl+Shift+T reabrir la cerrada. Clic con la rueda en una carpeta: la abre
  en otra pestaña; en una pestaña: la cierra.
- Se reordenan arrastrándolas. Archivos arrastrados sobre una pestaña: al
  soltarlos se mueven ahí; si esperas un momento, la pestaña se abre.
- Clic derecho: duplicar, abrir en ventana nueva, cerrar las demás o las de
  la derecha, reabrir y mandar a otro espacio de trabajo.
- Cada pestaña tiene su historial (atrás/adelante), su selección, su
  búsqueda y su vista. Las que no se ven guardan su lista de archivos (si la
  carpeta no es enorme) para volver al instante, y se refrescan por detrás.

### Espacios de trabajo
- Crea, renombra, elimina (tus carpetas no se tocan) y cambia de espacio
  con el botón a la izquierda de las pestañas o Ctrl+Shift+E.
- Cada espacio guarda sus pestañas y la vista de cada una, su color y
  logotipo y, si quieres, **su propio tema** y **sus propios ajustes** de
  archivos y de la órbita. Al cambiar de espacio, el tema cambia con su
  transición.
- **Restaurar sesión**: al abrir Solar vuelves al último espacio con sus
  pestañas (*Ajustes → Archivos*, se puede apagar).
- Se guardan en `~/.config/solar/espacios.lua`.
- Un espacio solo se abre en una ventana a la vez. Otra ventana usa un
  espacio **temporal** (borde punteado) que no se guarda, salvo que elijas
  *Guardar como espacio…*. Al guardar, cada ventana vuelve a leer el archivo
  y solo cambia lo suyo, así no se pisan.

### Otros cambios
- Soltar archivos en el fondo de la vista (no en una carpeta) los mueve a la
  carpeta abierta, y la vista se marca mientras pasan por encima. Antes solo
  pasaba con archivos que venían de otro programa.
- Si un atajo tuyo choca con uno nuevo (por ejemplo, tenías Ctrl+T para la
  terminal), gana el tuyo, el nuevo se queda sin atajo y Solar te avisa una
  vez.
- `config.lua` acepta `vista = "mosaicos"` y `"orbita"`, `orbita_girar`,
  `orbita_velocidad`, `orbita_orden`, `orbita_electricidad` y
  `restaurar_sesion`. Los `config.lua` de antes siguen sirviendo tal cual.

### Para desarrolladores
- `src/vista.rs` es el motor común y cada vista vive en `src/vista/`
  (implementa el trait `Disposicion`). Nuevos: `src/pestanas.rs`,
  `src/espacios.rs` y `nucleo/src/espacios.c`.
- 251 pruebas del núcleo (antes 226) y 21 de Rust (antes 16).

## v0.1.1-alpha «Viento solar» — 2026-11-09

Versión de optimización: mismo Solar, gastando menos (y, de paso, arrastrar
entre ventanas). Medido con una carpeta de 20 300 archivos (`SOLAR_PERFIL=1`
muestra estos tiempos).

### Arrastrar entre ventanas
- Ahora puedes sacar archivos de Solar arrastrándolos: a **otra ventana de
  Solar** (se mueven a la carpeta abierta, o a la carpeta o lugar donde los
  sueltes; si van a otro disco, se copian) o a una **terminal**, que escribe
  sus rutas ya entre comillas. También sirve con otros gestores de archivos
  y programas que acepten archivos.
- Soltar archivos *en* Solar ya funciona también en **Wayland** (antes solo
  en X11), y ahora cae donde lo sueltas: sobre una carpeta, un lugar de la
  barra lateral o una memoria USB, igual que al arrastrar dentro de Solar.
  Mientras pasa por encima, la carpeta se ilumina y aparece «Soltar aquí».
- Si el programa de origen solo permite copiar (un navegador, por ejemplo),
  se copia en vez de mover.
- En Wayland, mientras arrastras se ve un dibujito del archivo o la carpeta
  junto al puntero, con cuántos llevas si son varios.
- Por dentro: winit todavía no sabe empezar arrastres (ni recibirlos en
  Wayland), así que Solar habla directo con el servidor gráfico —
  `wl_data_device` en Wayland y el protocolo XDND en X11—, cada uno en un
  hilo que duerme hasta que pasa algo.

### Menos trabajo en reposo
- El hilo que vigila carpetas, papelera y discos ya no se despierta 4 veces
  por segundo: duerme hasta que pasa algo (inotify, mountinfo y ahora también
  `/dev` para memorias USB, con un eventfd para despertarlo). Despertares en
  reposo: de 6 a 2 por segundo (los 2 que quedan son de la accesibilidad).
- Los avisos ya no redibujan la ventana 20 veces por segundo mientras están
  quietos: solo mientras entran o se desvanecen.

### Carpetas grandes
- La barra de estado con 20 000 archivos seleccionados tardaba 3 ms en cada
  cuadro: ahora el texto se arma solo cuando cambia la selección (0,02 ms).
  Mover el mouse con todo seleccionado: de ~31 % a ~19 % de un núcleo.
- Ordenar por nombre es casi el doble de rápido (camino rápido ASCII en la
  comparación natural): 20 300 archivos en ~8 ms en vez de ~15 ms.
- El índice de rutas se arma en el hilo que lee la carpeta (antes trababa la
  ventana ~5 ms) y la carpeta vieja se suelta en otro hilo.
- Al salir de una carpeta enorme, la memoria se le devuelve al sistema
  (`malloc_trim`) y se limitan las arenas de malloc: 140 MB → 127 MB al
  volver a una carpeta chica.
- La descripción del tipo ("Imagen PNG"…) se comparte entre archivos y el
  nombre "plegado" para buscar ya no se guarda dos veces.
- Si una carpeta grande cambia sin parar (copiando miles de archivos), se
  relee como mucho una vez por segundo en vez de cada 150 ms.

### Arranque y ventana
- Revisar si la papelera está vacía (que recorre los discos montados, y en un
  disco de red puede tardar) ya no se hace en el hilo de la ventana.
- Comparar si cambió el tema en cada cuadro ya no copia el tema ni arma
  textos; dibujar cada archivo ya no copia su nombre, ruta y estilo (las
  entradas son compartidas).
- Los íconos de los programas de «Abrir con» se buscan y dibujan en un hilo
  aparte (antes, la primera vez, el menú podía trabarse).
- Con los sonidos apagados, ni se decodifican.

### Sonido
- Un solo hilo mezclador con una sola conexión a PulseAudio/PipeWire (o
  ALSA), con búfer de 60 ms: los clics suenan antes y ya no se abre un hilo y
  una conexión por cada sonido. Se cierra tras 2 s de silencio. Si suenan
  varios a la vez, se comprimen suave en vez de saturarse.

### Archivos
- Copiar a memorias FAT (sin `copy_file_range`) reutiliza el búfer de 1 MiB
  en vez de pedirlo y soltarlo por cada archivo.

### Para desarrolladores
- `SOLAR_PERFIL=1` mide cada parte del cuadro, el arranque, los listados y
  las revisiones de discos.
- 226 pruebas del núcleo (antes 218) y 16 de Rust (antes 14).

## v0.1.0-alpha «Amanecer» — 2026-11-08

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
  automática una vez al día, que se puede apagar. También detecta **commits
  nuevos en la rama principal** aunque no subas la versión: Solar sabe de
  qué commit salió (`solar --version` lo muestra) y los compara con GitHub.
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
