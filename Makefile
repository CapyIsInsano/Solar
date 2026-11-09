# ☀ Solar — atajos para compilar, probar e instalar
#
#   make                         compila (cargo build --release)
#   sudo make instalar           instala en /usr/local
#   make instalar PREFIX=~/.local   instala solo para tu usuario (sin sudo)
#   make desinstalar             lo quita (usa el mismo PREFIX)
#   make pruebas                 pruebas de Rust (incluyen el núcleo en C)
#   make pruebas-nucleo          pruebas del núcleo con AddressSanitizer
#                                (necesitan el Lua del sistema o LUA_DIR=…)
#
# Licencia: GPL-3.0-or-later

PREFIX  ?= /usr/local
DESTDIR ?=
CARGO   ?= cargo
FEATURES ?=

BIN      := target/release/solar
BINDIR   := $(DESTDIR)$(PREFIX)/bin
APPSDIR  := $(DESTDIR)$(PREFIX)/share/applications
ICONDIR  := $(DESTDIR)$(PREFIX)/share/icons/hicolor
DATADIR  := $(DESTDIR)$(PREFIX)/share/solar
TAMANOS  := 16 24 32 48 64 128 256 512

.PHONY: all compilar instalar desinstalar caches pruebas pruebas-nucleo limpiar \
        install uninstall test clean

all: compilar

compilar:
	$(CARGO) build --release $(FEATURES)

$(BIN):
	@echo "Primero compila Solar con: make"; exit 1

instalar: $(BIN)
	install -Dm755 $(BIN) $(BINDIR)/solar
	install -Dm644 recursos/solar.desktop $(APPSDIR)/solar.desktop
	install -Dm644 recursos/solar.svg $(ICONDIR)/scalable/apps/solar.svg
	for t in $(TAMANOS); do \
		install -Dm644 recursos/iconos/solar-$$t.png $(ICONDIR)/$${t}x$$t/apps/solar.png; done
	install -Dm644 config/config.lua $(DATADIR)/config.lua
	install -Dm644 config/temas/ejemplo.lua $(DATADIR)/temas/ejemplo.lua
	@$(MAKE) -s caches
	@echo ""
	@echo "☀ Solar quedó instalado en $(BINDIR)/solar"
	@echo "  Ábrelo desde tu lanzador o escribe: solar"

desinstalar:
	rm -f $(BINDIR)/solar $(APPSDIR)/solar.desktop
	rm -f $(ICONDIR)/scalable/apps/solar.svg
	for t in $(TAMANOS); do rm -f $(ICONDIR)/$${t}x$$t/apps/solar.png; done
	rm -rf $(DATADIR)
	@$(MAKE) -s caches
	@echo "Solar se desinstaló. Tu configuración sigue en ~/.config/solar"

# Actualiza los índices de lanzadores e íconos (si esas herramientas existen)
caches:
	@if command -v update-desktop-database >/dev/null 2>&1 && [ -d $(APPSDIR) ]; then \
		update-desktop-database -q $(APPSDIR) || true; fi
	@if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -d $(ICONDIR) ]; then \
		gtk-update-icon-cache -q -t $(ICONDIR) 2>/dev/null || true; fi

pruebas:
	$(CARGO) test $(FEATURES)

pruebas-nucleo:
	$(MAKE) -C nucleo pruebas

limpiar:
	$(CARGO) clean
	$(MAKE) -C nucleo clean

# Los mismos en inglés, por costumbre
install: instalar
uninstall: desinstalar
test: pruebas
clean: limpiar
