# -----------------------------------------------------------------------------
#  PROJECT STRUCTURE
# -----------------------------------------------------------------------------

OBJDIR := build
SRCDIR := src
INCDIR := include

OUTPUT ?= bin

SOURCES := $(shell find $(SRCDIR) -name '*.c')
OBJECTS := $(patsubst $(SRCDIR)/%.c, $(OBJDIR)/%.o, $(SOURCES))

# -----------------------------------------------------------------------------
#  COMPILATION
# -----------------------------------------------------------------------------

EXTRA_INC ?=
EXTRA_LIB ?=

CC ?= gcc
FFLAGS := -fsanitize=address -g
LDFLAGS := $(EXTRA_LIB) -lraylib -lm

$(OUTPUT): $(OBJECTS)
	@$(CC) $^ $(LDFLAGS) $(FFLAGS) -o $(OUTPUT)

$(OBJDIR)/%.o: $(SRCDIR)/%.c
	@mkdir -p $(dir $@)
	@$(CC) $(FFLAGS) -c $< -o $@ -I$(INCDIR) $(EXTRA_INC)

# -----------------------------------------------------------------------------
#  DEPLOY
# -----------------------------------------------------------------------------

web:
	emcc -Wall -Os -o ../site/index.html $(SOURCES) -I$(INCDIR) \
		-lm \
		$(RAYLIB)/libraylib.web.a -I$(RAYLIB) -L$(RAYLIB)/libraylib.web.a \
		-s USE_GLFW=3 -s ASYNCIFY -DPLATFORM_WEB --preload-file assets -DGRAPHICS_API_OPENGL_100 \
		--shell-file html/index.html

# -----------------------------------------------------------------------------
#  MAINTENANCE
# -----------------------------------------------------------------------------

.PHONY: clean run

run: $(OUTPUT)
	@./$(OUTPUT)

clean:
	@rm -rf $(OBJDIR) $(OUTPUT)
