# ☀ Solar — núcleo en C
#
# Normalmente el núcleo lo compila cargo (ver build.rs en la raíz). Este
# Makefile es para trabajar solo en el núcleo y correr sus pruebas:
#
#   make                    # build/libsolar_nucleo.a con el Lua del sistema
#   make pruebas            # pruebas con AddressSanitizer + UBSan
#   make pruebas LUA_DIR=/ruta/a/lua-5.4/src   # usando las fuentes de Lua
#
# Licencia: GPL-3.0-or-later

CC     ?= cc
CFLAGS ?= -O2 -g
CFLAGS += -std=c11 -D_GNU_SOURCE -Wall -Wextra -Wno-unused-parameter -Iinclude

ifdef LUA_DIR
  LUA_CFLAGS := -I$(LUA_DIR)
  LUA_SRC    := $(filter-out $(LUA_DIR)/lua.c $(LUA_DIR)/luac.c $(LUA_DIR)/onelua.c,$(wildcard $(LUA_DIR)/*.c))
  LUA_OBJ    := $(patsubst $(LUA_DIR)/%.c,build/lua/%.o,$(LUA_SRC))
  LUA_LIBS   := -lm
else
  LUA_PC     := $(shell for p in lua5.4 lua54 lua-5.4 lua; do pkg-config --exists $$p 2>/dev/null && echo $$p && break; done)
  LUA_CFLAGS := $(shell pkg-config --cflags $(LUA_PC) 2>/dev/null)
  LUA_LIBS   := $(shell pkg-config --libs $(LUA_PC) 2>/dev/null)
  LUA_OBJ    :=
endif

SRC := $(wildcard src/*.c)
OBJ := $(patsubst src/%.c,build/%.o,$(SRC))
SAN := -fsanitize=address,undefined -fno-omit-frame-pointer

.PHONY: all pruebas clean

all: build/libsolar_nucleo.a

build build/lua build/san:
	mkdir -p $@

build/%.o: src/%.c src/interno.h include/solar.h | build
	$(CC) $(CFLAGS) $(LUA_CFLAGS) -c $< -o $@

build/libsolar_nucleo.a: $(OBJ)
	$(AR) rcs $@ $^

build/lua/%.o: $(LUA_DIR)/%.c | build/lua
	$(CC) -O1 -g -DLUA_USE_POSIX $(SAN) -w -c $< -o $@

build/san/%.o: src/%.c src/interno.h include/solar.h | build/san
	$(CC) $(CFLAGS) $(SAN) $(LUA_CFLAGS) -c $< -o $@

build/pruebas: pruebas/pruebas.c $(patsubst src/%.c,build/san/%.o,$(SRC)) $(LUA_OBJ)
	$(CC) $(CFLAGS) -Wno-format-truncation $(SAN) $(LUA_CFLAGS) $^ $(LUA_LIBS) -lpthread -o $@

pruebas: build/pruebas
	./build/pruebas

clean:
	rm -rf build
