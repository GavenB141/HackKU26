# -----------------------------------------------------------------------------
#  PROJECT STRUCTURE
# -----------------------------------------------------------------------------

OBJDIR := build
SRCDIR := src
INCDIR := include

OUTPUT := bin

SOURCES := $(shell find $(SRCDIR) -name '*.c')
OBJECTS := $(patsubst $(SRCDIR)/%.c, $(OBJDIR)/%.o, $(SOURCES))

# -----------------------------------------------------------------------------
#  COMPILATION
# -----------------------------------------------------------------------------

CC := gcc
FFLAGS := -fsanitize=address
LDFLAGS := -lraylib

$(OUTPUT): $(OBJECTS)
	@$(CC) $^ $(LDFLAGS) $(FFLAGS) -o $(OUTPUT)

$(OBJDIR)/%.o: $(SRCDIR)/%.c
	@mkdir -p $(dir $@)
	@$(CC) $(FFLAGS) -c $< -o $@ -I$(INCDIR)

# -----------------------------------------------------------------------------
#  MAINTENANCE
# -----------------------------------------------------------------------------

.PHONY: clean run

run: $(OUTPUT)
	@./$(OUTPUT)

clean:
	@rm -rf $(OBJDIR) $(OUTPUT)
