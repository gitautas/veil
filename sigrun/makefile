include ./common/utils.mk

UNAME = $(shell uname)
TARGET = sigrun

CC = gcc
CFLAGS = -Wall -I./inc -ggdb
LIBS = -lX11 -lXdmcp -lGL
ifeq ($(UNAME), Linux)
	LIBS += -ldl
endif

SOURCES = src/sigrun.c ./common/NvFBCUtils.c
OBJECTS = $(call BUILD_OBJECT_LIST,$(SOURCES))
HEADERS = ./inc/NvFBCUtils.h ./inc/nvEncodeAPI.h ./inc/cuda_drvapi_dynlink_cuda.h

.PRECIOUS: $(TARGET) $(OBJECTS)
.PHONY: default all clean

default: $(TARGET)
all: default

$(foreach src,$(SOURCES),$(eval $(call DEFINE_OBJECT_RULE,$(src),$(HEADERS))))

$(TARGET): $(OBJECTS)
	$(CC) -m$(OS_SIZE) $(OBJECTS) $(LIBS) -o $@

clean:
	-rm -f *.o
	-rm -f $(TARGET)
