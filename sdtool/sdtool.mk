# compas-sdtool: the Compás player's library code as a command-line tool for a
# computer (see docs/SDTOOL.md). It builds against a compas-player checkout without
# changing it: run make in the player directory with the player Makefile first, which
# supplies the vendored library variables and flags. All output goes under
# SDTOOL_DIR in this repository.
#
#   make -C "$PLAYER" -f Makefile -f "$TOOLS/sdtool/sdtool.mk" sdtool
#                                native build -> build/sdtool/compas-sdtool
#   ... sdtool SDTOOL_TARGET=windows
#                                MinGW-w64 build -> build/sdtool-windows/compas-sdtool.exe
#   ... test-sdtool              end-to-end test on a generated card
#
# SDTOOL_TARGET=windows expects x86_64-w64-mingw32-gcc and a mingw cmake
# toolchain (or CMAKE_TOOLCHAIN_FILE in SDTOOL_CMAKE_EXTRA).

SDTOOL_ROOT := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
SDTOOL_TARGET ?= native
SDTOOL_SRC_DIR ?= $(SDTOOL_ROOT)/src

ifeq ($(SDTOOL_TARGET),windows)
SDTOOL_DIR ?= $(abspath $(SDTOOL_ROOT)/../build)/sdtool-windows
SDTOOL_CC ?= x86_64-w64-mingw32-gcc
SDTOOL_EXE = .exe
SDTOOL_PLATFORM_CFLAGS = -include $(SDTOOL_SRC_DIR)/compat/compat.h -I$(SDTOOL_SRC_DIR)/compat \
                         -D_FILE_OFFSET_BITS=64 -D__USE_MINGW_ANSI_STDIO=1 -D_WIN32_WINNT=0x0601
SDTOOL_PLATFORM_SRCS = $(SDTOOL_SRC_DIR)/compat/compat.c $(SDTOOL_SRC_DIR)/compat/compat_dirs.c
SDTOOL_PLATFORM_LIBS = -static -lbcrypt -lpthread
SDTOOL_CMAKE_FLAGS = -DCMAKE_SYSTEM_NAME=Windows -DCMAKE_C_COMPILER=$(SDTOOL_CC) $(SDTOOL_CMAKE_EXTRA)
else
SDTOOL_DIR ?= $(abspath $(SDTOOL_ROOT)/../build)/sdtool
SDTOOL_CC ?= $(CC)
SDTOOL_EXE =
# Linux and macOS: stat() is routed through mtime.c; macOS also needs compat.h.
SDTOOL_PLATFORM_CFLAGS = -include $(SDTOOL_SRC_DIR)/compat/stat_shim.h \
                         $(if $(filter Darwin,$(shell uname -s)),-include $(SDTOOL_SRC_DIR)/compat/compat.h)
SDTOOL_PLATFORM_SRCS = $(if $(filter Darwin,$(shell uname -s)),$(SDTOOL_SRC_DIR)/compat/compat_apple.c)
SDTOOL_PLATFORM_LIBS = -lpthread $(if $(filter Darwin,$(shell uname -s)),-framework CoreFoundation)
SDTOOL_CMAKE_FLAGS = -DCMAKE_C_COMPILER=$(SDTOOL_CC)
endif

SDTOOL_BIN = $(SDTOOL_DIR)/compas-sdtool$(SDTOOL_EXE)

SDTOOL_OWN_SRCS = $(addprefix $(SDTOOL_SRC_DIR)/,cli/main.c output/json_out.c index/root.c index/database.c index/scan.c media/art.c time/mtime.c time/mtime_calibrate.c parse/parse_pool.c index/process.c cli/stubs.c cli/version.c media/lvgl_shim.c media/dr_libs_impl.c) \
                  $(SDTOOL_PLATFORM_SRCS)
# The player's own library code, unchanged: tagcache and metadata_db for the
# database, albumart and cover_decode for the covers, metadata for the tags.
SDTOOL_PLAYER_SRCS = $(addprefix src/library/,tagcache/core/tagcache.c metadata/db/metadata_db.c metadata/parsing/metadata.c \
                       artwork/albumart/albumart.c browser/paths/path_cache.c remote/remote_state.c \
                       remote/subsonic/subsonic_saved_servers.c parsers/cue/cue_parser.c sacd/sacd_iso.c \
                       text/lyrics/lyrics.c artwork/decode/cover_decode.c artwork/coordinator/artwork_coordinator.c \
                       artwork/coordinator/artwork_memory.c metadata/scan/library_walk.c metadata/scan/library_scan_tags.c) \
                     src/core/system/logging/db_log.c src/core/text/utf8_util.c \
                     $(addprefix src/audio/,decoders/ogg/ogg_probe.c decoders/ogg/opusfile_alloc.c decoders/wavpack/wavpack_decoder.c decoders/caf/caf_decoder.c)
SDTOOL_LVGL_SRCS = $(LVGL_DIR)/src/libs/lodepng/lodepng.c $(LVGL_DIR)/src/libs/tjpgd/tjpgd.c
SDTOOL_MBEDTLS_SRCS = $(MBEDTLS_DIR)/library/base64.c $(MBEDTLS_DIR)/library/platform_util.c

# HOST_BUILD drops the player's SD-mount check and its device-only paths; the
# card root is chosen at run time (storage_paths.h).
SDTOOL_CFLAGS = $(CFLAGS) -DHOST_BUILD=1 -DCOMPAS_RUNTIME_SD_ROOT=1 -DARTWORK_FALLBACK_MEM_BYTES=1073741824U \
                -I$(TINFL_DIR) -DMINIZ_NO_DEFLATE_APIS -DMINIZ_NO_ARCHIVE_APIS -Isrc -I$(SDTOOL_SRC_DIR) \
                $(SDTOOL_PLATFORM_CFLAGS)

SDTOOL_OBJS = $(SDTOOL_OWN_SRCS:$(SDTOOL_SRC_DIR)/%.c=$(SDTOOL_DIR)/own/%.o) $(SDTOOL_PLAYER_SRCS:%.c=$(SDTOOL_DIR)/%.o) \
              $(SDTOOL_LVGL_SRCS:%.c=$(SDTOOL_DIR)/%.o) $(SDTOOL_MBEDTLS_SRCS:%.c=$(SDTOOL_DIR)/%.o) \
              $(CJSON_SRCS:%.c=$(SDTOOL_DIR)/%.o) $(STB_VORBIS_SRCS:%.c=$(SDTOOL_DIR)/%.o) \
              $(TINFL_SRCS:%.c=$(SDTOOL_DIR)/%.o) $(OPUS_SRCS:%.c=$(SDTOOL_DIR)/%.o) \
              $(OGG_SRCS:%.c=$(SDTOOL_DIR)/%.o) $(OPUSFILE_SRCS:%.c=$(SDTOOL_DIR)/%.o) \
              $(JPEG_SRCS:%.c=$(SDTOOL_DIR)/%.o)
SDTOOL_LIBS = $(SDTOOL_DIR)/libwavpack/libwavpack.a $(SDTOOL_DIR)/libsndfile/libsndfile.a

# The id reported by `info`: this repository's revision and the player's, so a bundled
# tool can be traced to its sources. Callers that cannot run git here pass SDTOOL_BUILD_ID.
# A revision is marked -dirty only when git ran; untracked files count under $(2) too.
SDTOOL_REV = $(shell git -C $(1) rev-parse --short HEAD 2>/dev/null || echo unknown)$(if $(shell git -C $(1) rev-parse HEAD 2>/dev/null),$(if $(shell git -C $(1) status --porcelain -- $(2) 2>/dev/null),-dirty))
SDTOOL_BUILD_ID := $(or $(SDTOOL_BUILD_ID),tools-$(call SDTOOL_REV,$(SDTOOL_ROOT),.)+player-$(call SDTOOL_REV,.,.))
.PHONY: SDTOOL_FORCE
$(SDTOOL_DIR)/.build_id: SDTOOL_FORCE
	@mkdir -p $(dir $@)
	@printf '%s\n' '$(SDTOOL_BUILD_ID)' > $@.tmp
	@cmp -s $@.tmp $@ && rm $@.tmp || mv $@.tmp $@
$(SDTOOL_DIR)/own/cli/version.o: $(SDTOOL_DIR)/.build_id
$(SDTOOL_DIR)/own/cli/version.o: SDTOOL_CFLAGS += -DSDTOOL_BUILD=\"$(SDTOOL_BUILD_ID)\"

.PHONY: sdtool
sdtool: $(SDTOOL_BIN)

$(SDTOOL_BIN): $(SDTOOL_OBJS) $(SDTOOL_LIBS)
	$(SDTOOL_CC) -o $@ $(SDTOOL_OBJS) $(SDTOOL_LIBS) $(SDTOOL_PLATFORM_LIBS) -lm

# Vendored libraries keep the flags the player builds them with.
$(SDTOOL_DIR)/$(OPUS_DIR)/%.o: $(OPUS_DIR)/%.c $(OPUS_WARNING_STAMP)
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(OPUS_CFLAGS) -c $< -o $@
$(SDTOOL_DIR)/$(OGG_DIR)/%.o: $(OGG_DIR)/%.c libogg_vendor_config/ogg/config_types.h
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(OGG_CFLAGS) -c $< -o $@
$(SDTOOL_DIR)/$(OPUSFILE_DIR)/%.o: $(OPUSFILE_DIR)/%.c libogg_vendor_config/ogg/config_types.h $(OPUSFILE_PATCH_STAMP) $(OPUSFILE_WARN_STAMP)
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(OPUSFILE_CFLAGS) -c $< -o $@
$(SDTOOL_DIR)/$(JPEG_DIR)/%.o: $(JPEG_DIR)/%.c
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(JPEG_CFLAGS) -c $< -o $@
$(SDTOOL_DIR)/own/%.o: $(SDTOOL_SRC_DIR)/%.c $(LVGL_PATCH_STAMP)
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(SDTOOL_CFLAGS) -c $< -o $@
$(SDTOOL_DIR)/%.o: %.c $(LVGL_PATCH_STAMP)
	@mkdir -p $(dir $@)
	$(SDTOOL_CC) $(SDTOOL_CFLAGS) -c $< -o $@

$(SDTOOL_DIR)/libwavpack/libwavpack.a: $(WAVPACK_DIR)/CMakeLists.txt $(WAVPACK_CMAKE_STAMP)
	cmake -S $(WAVPACK_DIR) -B $(dir $@) $(WAVPACK_OPTIONS) $(SDTOOL_CMAKE_FLAGS)
	+cmake --build $(dir $@) --target wavpack
$(SDTOOL_DIR)/libsndfile/libsndfile.a: $(SNDFILE_DIR)/CMakeLists.txt $(SNDFILE_CMAKE_STAMP)
	cmake -S $(SNDFILE_DIR) -B $(dir $@) $(SNDFILE_OPTIONS) $(SDTOOL_CMAKE_FLAGS)
	+cmake --build $(dir $@) --target sndfile

-include $(SDTOOL_OBJS:.o=.d)

.PHONY: test-sdtool
# End to end on a generated card: scan, art, no-write rescans, the file-time rule.
test-sdtool: $(SDTOOL_BIN)
	sh $(SDTOOL_ROOT)/tests/run_sdtool_test.sh $(SDTOOL_BIN)

ifeq ($(SDTOOL_TARGET),windows)
# Windows only: the compat layer's folder pattern for drive roots (run under Wine).
$(SDTOOL_DIR)/compat_pattern_test.exe: $(SDTOOL_ROOT)/tests/compat_pattern_test.c $(SDTOOL_DIR)/own/compat/compat.o \
                                      $(SDTOOL_DIR)/own/compat/compat_dirs.o $(SDTOOL_DIR)/own/time/mtime.o
	$(SDTOOL_CC) $(SDTOOL_CFLAGS) -o $@ $^ -static -lbcrypt -lpthread
.PHONY: sdtool-compat-test
sdtool-compat-test: $(SDTOOL_DIR)/compat_pattern_test.exe
endif
