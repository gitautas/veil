#include <dlfcn.h>
#include <getopt.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <GL/gl.h>
#include <GL/glx.h>
#include <X11/Xlib.h>

#include "NvFBC.h"
#include "NvFBCUtils.h"
#include "nvEncodeAPI.h"

#define APP_VERSION 1

#define LIB_NVFBC_NAME "libnvidia-fbc.so.1"
#define LIB_ENCODEAPI_NAME "libnvidia-encode.so.1"

/*
 * Global variables
 */
GLXContext glxCtx = None;
GLXFBConfig glxFBConfig = None;
NV_ENCODE_API_FUNCTION_LIST pEncFn;

/*
 * NvEncodeAPI entry point
 */
typedef NVENCSTATUS(NVENCAPI *PFNNVENCODEAPICREATEINSTANCEPROC)(
    NV_ENCODE_API_FUNCTION_LIST *);

/**
 * Creates an OpenGL context.
 *
 * This context will then be passed to NvFBC for its internal use.
 *
 * \param [out] *glxCtx
 *   The created OpenGL context.
 * \param [out] *glxFbConfig
 *   The used framebuffer configuration.
 *
 * \return
 *   NVFBC_TRUE in case of success, NVFBC_FALSE otherwise.
 */
static NVFBC_BOOL gl_init(void) {
  Display *dpy = None;
  Pixmap pixmap = None;
  GLXPixmap glxPixmap = None;
  GLXFBConfig *fbConfigs;
  Bool res;
  int n;

  int attribs[] = {GLX_DRAWABLE_TYPE,
                   GLX_PIXMAP_BIT | GLX_WINDOW_BIT,
                   GLX_BIND_TO_TEXTURE_RGBA_EXT,
                   1,
                   GLX_BIND_TO_TEXTURE_TARGETS_EXT,
                   GLX_TEXTURE_2D_BIT_EXT,
                   None};

  dpy = XOpenDisplay(NULL);
  if (dpy == None) {
    fprintf(stderr, "Unable to open display\n");
    return NVFBC_FALSE;
  }

  fbConfigs = glXChooseFBConfig(dpy, DefaultScreen(dpy), attribs, &n);
  if (!fbConfigs) {
    fprintf(stderr, "Unable to find FB configs\n");
    return NVFBC_FALSE;
  }

  glxCtx = glXCreateNewContext(dpy, fbConfigs[0], GLX_RGBA_TYPE, None, True);
  if (glxCtx == None) {
    fprintf(stderr, "Unable to create GL context\n");
    return NVFBC_FALSE;
  }

  pixmap = XCreatePixmap(dpy, XDefaultRootWindow(dpy), 1, 1,
                         DisplayPlanes(dpy, XDefaultScreen(dpy)));
  if (pixmap == None) {
    fprintf(stderr, "Unable to create pixmap\n");
    return NVFBC_FALSE;
  }

  glxPixmap = glXCreatePixmap(dpy, fbConfigs[0], pixmap, NULL);
  if (glxPixmap == None) {
    fprintf(stderr, "Unable to create GLX pixmap\n");
    return NVFBC_FALSE;
  }

  res = glXMakeCurrent(dpy, glxPixmap, glxCtx);
  if (!res) {
    fprintf(stderr, "Unable to make context current\n");
    return NVFBC_FALSE;
  }

  glxFBConfig = fbConfigs[0];

  XFree(fbConfigs);

  return NVFBC_TRUE;
}

/**
 * Checks that the requested codec is supported by the HW encoder
 *
 * \param [in] *encoder
 *   The handle to the encoder instance.
 * \param [in] encodeGUID
 *   The GUID corresponding to the codec requested.
 *
 * \return
 *   NV_ENC_SUCCESS in case of success, error code otherwise.
 */
static NVENCSTATUS validateEncodeGUID(void *encoder, GUID encodeGuid) {
  unsigned int nGuids = 0, i, encodeGuidCount = 0, codecFound = 0;
  GUID *encodeGuidArray = NULL;
  NVENCSTATUS status = NV_ENC_SUCCESS;

  status = pEncFn.nvEncGetEncodeGUIDCount(encoder, &encodeGuidCount);
  if (status != NV_ENC_SUCCESS) {
    fprintf(stderr,
            "Failed to query number of supported codecs, "
            "status = %d\n",
            status);
    goto fail;
  }

  encodeGuidArray = (GUID *)malloc(sizeof(GUID) * encodeGuidCount);
  if (!encodeGuidArray) {
    fprintf(stderr, "Failed to allocate GUID array, status = %d\n", status);
    goto fail;
  }

  status = pEncFn.nvEncGetEncodeGUIDs(encoder, encodeGuidArray, encodeGuidCount,
                                      &nGuids);
  if (status != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to query supported codecs, status = %d\n", status);
    goto fail;
  }

  for (i = 0; i < nGuids; i++) {
    if (!memcmp(&encodeGuid, &encodeGuidArray[i], sizeof(GUID))) {
      codecFound = 1;
      break;
    }
  }

fail:
  if (encodeGuidArray) {
    free(encodeGuidArray);
    encodeGuidArray = NULL;
  }

  return codecFound ? NV_ENC_SUCCESS : status;
}

/**
 * Initializes the NvFBC library and creates an NvFBC instance.
 *
 * Creates and sets up a capture session to video memory.
 *
 * Creates and sets up an encode session and initializes the encoder with
 * parameters for generating an H.264 stream.
 *
 * Captures frames, encodes them and writes them out to a file (output.h264)
 */
int main(int argc, char *argv[]) {
  unsigned int n = 0, i;
  NVFBC_SIZE frameSize = {0, 0};

  FILE *outPipe;

  void *libNVFBC = NULL, *libEnc = NULL;

  PNVFBCCREATEINSTANCE NvFBCCreateInstance_ptr = NULL;
  PFNNVENCODEAPICREATEINSTANCEPROC NvEncodeAPICreateInstance = NULL;
  NVFBC_API_FUNCTION_LIST pFn;

  NVFBCSTATUS fbcStatus;
  NVFBC_BOOL fbcBool;
  NVENCSTATUS encStatus;

  NVFBC_SESSION_HANDLE fbcHandle;
  NVFBC_CREATE_HANDLE_PARAMS createHandleParams;
  NVFBC_DESTROY_HANDLE_PARAMS destroyHandleParams;

  void *encoder = NULL;

  printf("Hello from c!\n");

  /*
   * Parse the command line.
   */
  if (argc < 2) {
    fprintf(stderr, "No named pipe specified\n");
    return EXIT_FAILURE;
  }
  outPipe = fopen(argv[1], "w");

  /*
   * Dynamically load the NvFBC library.
   */
  libNVFBC = dlopen(LIB_NVFBC_NAME, RTLD_NOW);
  if (libNVFBC == NULL) {
    fprintf(stderr, "Unable to open '%s'\n", LIB_NVFBC_NAME);
    return EXIT_FAILURE;
  }

  /*
   * Dynamically load the NvEncodeAPI library.
   */
  libEnc = dlopen(LIB_ENCODEAPI_NAME, RTLD_NOW);
  if (libNVFBC == NULL) {
    fprintf(stderr, "Unable to open '%s'\n", LIB_ENCODEAPI_NAME);
    return EXIT_FAILURE;
  }

  /*
   * Initialize OpenGL.
   */
  fbcBool = gl_init();
  if (fbcBool != NVFBC_TRUE) {
    return EXIT_FAILURE;
  }

  /*
   * Resolve the 'NvFBCCreateInstance' symbol that will allow us to get
   * the API function pointers.
   */
  NvFBCCreateInstance_ptr =
      (PNVFBCCREATEINSTANCE)dlsym(libNVFBC, "NvFBCCreateInstance");
  if (NvFBCCreateInstance_ptr == NULL) {
    fprintf(stderr, "Unable to resolve symbol 'NvFBCCreateInstance'\n");
    return EXIT_FAILURE;
  }

  /*
   * Create an NvFBC instance.
   *
   * API function pointers are accessible through pFn.
   */
  memset(&pFn, 0, sizeof(pFn));

  pFn.dwVersion = NVFBC_VERSION;

  fbcStatus = NvFBCCreateInstance_ptr(&pFn);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "Unable to create NvFBC instance (status: %d)\n",
            fbcStatus);
    return EXIT_FAILURE;
  }

  /*
   * Resolve the 'NvEncodeAPICreateInstance' symbol that will allow us to get
   * the API function pointers.
   */
  NvEncodeAPICreateInstance = (PFNNVENCODEAPICREATEINSTANCEPROC)dlsym(
      libEnc, "NvEncodeAPICreateInstance");
  if (NvEncodeAPICreateInstance == NULL) {
    fprintf(stderr, "Unable to resolve symbol 'NvEncodeAPICreateInstance'\n");
    return EXIT_FAILURE;
  }

  /*
   * Create an NvEncodeAPI instance.
   *
   * API function pointers are accessible through pEncFn.
   */
  memset(&pEncFn, 0, sizeof(pEncFn));

  pEncFn.version = NV_ENCODE_API_FUNCTION_LIST_VER;

  encStatus = NvEncodeAPICreateInstance(&pEncFn);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Unable to create NvEncodeAPI instance (status: %d)\n",
            encStatus);
    return EXIT_FAILURE;
  }

  /*
   * Create a session handle that is used to identify the client.
   *
   * Request that the GL context is externally managed.
   */
  memset(&createHandleParams, 0, sizeof(createHandleParams));

  createHandleParams.dwVersion = NVFBC_CREATE_HANDLE_PARAMS_VER;
  createHandleParams.bExternallyManagedContext = NVFBC_TRUE;
  createHandleParams.glxCtx = glxCtx;
  createHandleParams.glxFBConfig = glxFBConfig;

  fbcStatus = pFn.nvFBCCreateHandle(&fbcHandle, &createHandleParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
    return EXIT_FAILURE;
  }

  NVFBC_GET_STATUS_PARAMS statusParams;
  NVFBC_CREATE_CAPTURE_SESSION_PARAMS createCaptureParams;
  NVFBC_TOGL_SETUP_PARAMS setupParams;
  NVFBC_DESTROY_CAPTURE_SESSION_PARAMS destroyCaptureParams;

  NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS encodeSessionParams;
  NV_ENC_PRESET_CONFIG presetConfig;
  NV_ENC_INITIALIZE_PARAMS initParams;
  NV_ENC_REGISTERED_PTR registeredResources[NVFBC_TOGL_TEXTURES_MAX] = {NULL};
  NV_ENC_MAP_INPUT_RESOURCE mapParams;
  NV_ENC_PIC_PARAMS encParams;
  NV_ENC_CREATE_BITSTREAM_BUFFER bitstreamBufferParams;
  NV_ENC_LOCK_BITSTREAM lockParams;

  NV_ENC_INPUT_PTR inputBuffer = NULL;
  NV_ENC_OUTPUT_PTR outputBuffer = NULL;
  int bufferSize = 0;

  /*
   * Retrieve the size of framebuffer.
   */
  memset(&statusParams, 0, sizeof(statusParams));

  statusParams.dwVersion = NVFBC_GET_STATUS_PARAMS_VER;

  fbcStatus = pFn.nvFBCGetStatus(fbcHandle, &statusParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
    goto fbc_fail;
  }

  if (statusParams.bCanCreateNow == NVFBC_FALSE) {
    fprintf(stderr, "It is not possible to create a capture session "
                    "on this system.\n");
    goto fbc_fail;
  }

  if (frameSize.w > statusParams.screenSize.w ||
      frameSize.h > statusParams.screenSize.h) {
    fprintf(stderr,
            "Frames larger than %dx%d cannot be captured on this "
            "system.\n",
            statusParams.screenSize.w, statusParams.screenSize.h);
    goto fbc_fail;
  }

  if (frameSize.w == 0) {
    /* frameSize.w = statusParams.screenSize.w; */
    frameSize.w = 1920;
    printf("Setting width to %d\n", frameSize.w);
  }

  if (frameSize.h == 0) {
    /* frameSize.h = statusParams.screenSize.h; */
    frameSize.h = 1080;
    printf("Setting height to %d\n", frameSize.h);
  }

  /*
   * The width is a multiple of 4 so that it can be used as the surface
   * pitch when calling NvEncRegisterResource
   */
  frameSize.w = (frameSize.w + 3) & ~3;

  /*
   * Create a capture session.
   */
  memset(&createCaptureParams, 0, sizeof(createCaptureParams));

  createCaptureParams.dwVersion = NVFBC_CREATE_CAPTURE_SESSION_PARAMS_VER;
  createCaptureParams.eCaptureType = NVFBC_CAPTURE_TO_GL;
  createCaptureParams.bWithCursor = NVFBC_TRUE;
  createCaptureParams.frameSize = frameSize;
  createCaptureParams.eTrackingType = NVFBC_TRACKING_DEFAULT;
  createCaptureParams.bDisableAutoModesetRecovery = NVFBC_TRUE;

  fbcStatus = pFn.nvFBCCreateCaptureSession(fbcHandle, &createCaptureParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
    goto fbc_fail;
  }

  /*
   * Set up the capture session.
   */
  memset(&setupParams, 0, sizeof(setupParams));

  setupParams.dwVersion = NVFBC_TOGL_SETUP_PARAMS_VER;
  setupParams.eBufferFormat = NVFBC_BUFFER_FORMAT_NV12;

  fbcStatus = pFn.nvFBCToGLSetUp(fbcHandle, &setupParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
    goto fbc_fail;
  }

  /*
   * Create an encoder session
   */
  memset(&encodeSessionParams, 0, sizeof(encodeSessionParams));

  encodeSessionParams.version = NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER;
  encodeSessionParams.apiVersion = NVENCAPI_VERSION;
  encodeSessionParams.deviceType = NV_ENC_DEVICE_TYPE_OPENGL;

  encStatus = pEncFn.nvEncOpenEncodeSessionEx(&encodeSessionParams, &encoder);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to open an encoder session, status = %d\n",
            encStatus);
    goto enc_fail;
  }

  /*
   * Validate the codec requested
   */
  GUID encodeGuid = NV_ENC_CODEC_H264_GUID;
  encStatus = validateEncodeGUID(encoder, encodeGuid);
  if (encStatus != NV_ENC_SUCCESS) {
    goto enc_fail;
  }

  memset(&presetConfig, 0, sizeof(presetConfig));

  presetConfig.version = NV_ENC_PRESET_CONFIG_VER;
  presetConfig.presetCfg.version = NV_ENC_CONFIG_VER;

  encStatus = pEncFn.nvEncGetEncodePresetConfig(
      encoder, encodeGuid, NV_ENC_PRESET_DEFAULT_GUID,
      &presetConfig);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr,
            "Failed to obtain preset settings, "
            "status = %d\n",
            encStatus);
    goto enc_fail;
  }

  presetConfig.presetCfg.profileGUID = NV_ENC_H264_PROFILE_BASELINE_GUID;
  presetConfig.presetCfg.rcParams.averageBitRate =
      20 * 1024 * 1024; /* Changed from 5 */
  presetConfig.presetCfg.rcParams.maxBitRate =
      30 * 1024 * 1024;                                   /* Changed from 8 */
  presetConfig.presetCfg.rcParams.vbvBufferSize = 0; /* single frame */

  /*
   * Initialize the encode session
   */
  memset(&initParams, 0, sizeof(initParams));

  initParams.version = NV_ENC_INITIALIZE_PARAMS_VER;
  initParams.encodeGUID = encodeGuid;
  initParams.presetGUID = NV_ENC_PRESET_DEFAULT_GUID;
  initParams.encodeConfig = &presetConfig.presetCfg;
  initParams.encodeWidth = frameSize.w;
  initParams.encodeHeight = frameSize.h;
  initParams.frameRateNum = 1; // This is a strange option, it does something but not setting the framerate...
  initParams.frameRateDen = 1;
  initParams.enablePTD = 1;

  printf("%d \n", frameSize.w);
  printf("%d \n", frameSize.h);

  encStatus = pEncFn.nvEncInitializeEncoder(encoder, &initParams);
  if (encStatus == NV_ENC_ERR_INVALID_PARAM) {
    printf("Invalid param :(\n");
  }

  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to initialize the encode session, status = %d\n",
            encStatus);
    goto enc_fail;
  }

  /*
   * Register the textures received from NvFBC for use with NvEncodeAPI
   */
  for (i = 0; i < NVFBC_TOGL_TEXTURES_MAX; i++) {
    NV_ENC_REGISTER_RESOURCE registerParams;
    NV_ENC_INPUT_RESOURCE_OPENGL_TEX texParams;

    if (!setupParams.dwTextures[i]) {
      break;
    }

    memset(&registerParams, 0, sizeof(registerParams));

    texParams.texture = setupParams.dwTextures[i];
    texParams.target = setupParams.dwTexTarget;

    registerParams.version = NV_ENC_REGISTER_RESOURCE_VER;
    registerParams.resourceType = NV_ENC_INPUT_RESOURCE_TYPE_OPENGL_TEX;
    registerParams.width = frameSize.w;
    registerParams.height = frameSize.h;
    registerParams.pitch = frameSize.w;
    registerParams.resourceToRegister = &texParams;
    registerParams.bufferFormat = NV_ENC_BUFFER_FORMAT_NV12;

    encStatus = pEncFn.nvEncRegisterResource(encoder, &registerParams);
    if (encStatus != NV_ENC_SUCCESS) {
      fprintf(stderr, "Failed to register texture, status = %d\n", encStatus);
      goto enc_fail;
    }

    registeredResources[i] = registerParams.registeredResource;
  }

  /*
   * Create a bitstream buffer to hold the output
   */
  memset(&bitstreamBufferParams, 0, sizeof(bitstreamBufferParams));
  bitstreamBufferParams.version = NV_ENC_CREATE_BITSTREAM_BUFFER_VER;

  encStatus =
      pEncFn.nvEncCreateBitstreamBuffer(encoder, &bitstreamBufferParams);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to create a bitstream buffer, status = %d\n",
            encStatus);
    goto enc_fail;
  }

  outputBuffer = bitstreamBufferParams.bitstreamBuffer;

  /*
   * Pre-fill mapping information
   */
  memset(&mapParams, 0, sizeof(mapParams));

  mapParams.version = NV_ENC_MAP_INPUT_RESOURCE_VER;

  /*
   * Pre-fill frame encoding information
   */
  memset(&encParams, 0, sizeof(encParams));

  encParams.version = NV_ENC_PIC_PARAMS_VER;
  encParams.inputWidth = frameSize.w;
  encParams.inputHeight = frameSize.h;
  encParams.inputPitch = frameSize.w;
  encParams.pictureStruct = NV_ENC_PIC_STRUCT_FRAME;
  encParams.outputBitstream = outputBuffer;

  /*
   * Start capturing and encoding frames.
   */
  for (;;) {
    NVFBC_TOGL_GRAB_FRAME_PARAMS grabParams;

    memset(&grabParams, 0, sizeof(grabParams));

    grabParams.dwVersion = NVFBC_TOGL_GRAB_FRAME_PARAMS_VER;

    /*
     * Capture a frame.
     */
    fbcStatus = pFn.nvFBCToGLGrabFrame(fbcHandle, &grabParams);
    if (fbcStatus == NVFBC_ERR_MUST_RECREATE) {
      printf("Capture session must be recreated!\n");
      break;
    } else if (fbcStatus != NVFBC_SUCCESS) {
      fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
      break;
    }

    /*
     * Map the frame for use by the encoder.
     */
    mapParams.registeredResource =
        registeredResources[grabParams.dwTextureIndex];
    encStatus = pEncFn.nvEncMapInputResource(encoder, &mapParams);
    if (encStatus != NV_ENC_SUCCESS) {
      fprintf(stderr, "Failed to map the resource, status = %d\n", encStatus);
      goto enc_fail;
    }

    encParams.inputBuffer = inputBuffer = mapParams.mappedResource;
    encParams.bufferFmt = mapParams.mappedBufferFmt;
    encParams.frameIdx = encParams.inputTimeStamp = n;

    /*
     * Encode the frame.
     */
    encStatus = pEncFn.nvEncEncodePicture(encoder, &encParams);
    if (encStatus != NV_ENC_SUCCESS) {
      fprintf(stderr, "Failed to encode frame, status = %d\n", encStatus);
    } else {
      /*
       * Get the bitstream and dump to file.
       */
      memset(&lockParams, 0, sizeof(lockParams));

      lockParams.version = NV_ENC_LOCK_BITSTREAM_VER;
      lockParams.outputBitstream = outputBuffer;

      encStatus = pEncFn.nvEncLockBitstream(encoder, &lockParams);
      if (encStatus == NV_ENC_SUCCESS) {
        bufferSize = lockParams.bitstreamSizeInBytes;
        fwrite(lockParams.bitstreamBufferPtr, 1, bufferSize, outPipe);

        encStatus = pEncFn.nvEncUnlockBitstream(encoder, outputBuffer);
        if (encStatus != NV_ENC_SUCCESS) {
          /*
           * We usually shouln't be here.
           */
          fprintf(stderr, "Failed to unlock bitstream buffer, status = %d\n",
                  encStatus);
        }
      } else {
        fprintf(stderr, "Failed to lock bitstream buffer, status = %d\n",
                encStatus);
      }
    }

    /*
     * Unmap the frame.
     */
    encStatus = pEncFn.nvEncUnmapInputResource(encoder, inputBuffer);
    if (encStatus != NV_ENC_SUCCESS) {
      fprintf(stderr, "Failed to unmap the resource, status = %d\n", encStatus);
      goto enc_fail;
    }

    if (bufferSize == 0) {
      /*
       * We failed to obtain the bitstream for some reason; it's better
       * to terminate.
       */
      goto enc_fail;
    }
  }

  printf("Captured %d frames.\n", n);
  printf("Destroying resources...\n");

enc_fail:
  /*
   * Flush the encoder. A no-op if no frames have been encoded
   */
  memset(&encParams, 0, sizeof(encParams));
  encParams.version = NV_ENC_PIC_PARAMS_VER;
  encParams.encodePicFlags = NV_ENC_PIC_FLAG_EOS;

  encStatus = pEncFn.nvEncEncodePicture(encoder, &encParams);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to flush the encoder, status = %d\n", encStatus);
  }

  /*
   * Deallocate the bitstream buffer
   */
  if (outputBuffer) {
    encStatus = pEncFn.nvEncDestroyBitstreamBuffer(encoder, outputBuffer);
    if (encStatus != NV_ENC_SUCCESS) {
      fprintf(stderr, "Failed to destroy buffer, status = %d\n", encStatus);
    }

    outputBuffer = NULL;
  }

  /*
   * Unregister all the resources that we had registered earlier with the
   * encoder.
   */
  for (i = 0; i < NVFBC_TOGL_TEXTURES_MAX; i++) {
    if (registeredResources[i]) {
      encStatus =
          pEncFn.nvEncUnregisterResource(encoder, registeredResources[i]);
      if (encStatus != NV_ENC_SUCCESS) {
        fprintf(stderr, "Failed to unregister resource, status = %d\n",
                encStatus);
      }
      registeredResources[i] = NULL;
    }
  }

  /*
   * Destroy the encode session
   */
  encStatus = pEncFn.nvEncDestroyEncoder(encoder);
  if (encStatus != NV_ENC_SUCCESS) {
    fprintf(stderr, "Failed to destroy encoder, status = %d\n", encStatus);
  }

fbc_fail:
  /*
   * Destroy capture session.
   */
  memset(&destroyCaptureParams, 0, sizeof(destroyCaptureParams));

  destroyCaptureParams.dwVersion = NVFBC_DESTROY_CAPTURE_SESSION_PARAMS_VER;

  fbcStatus = pFn.nvFBCDestroyCaptureSession(fbcHandle, &destroyCaptureParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
  }

  /*
   * Destroy session handle, tear down more resources.
   */
  memset(&destroyHandleParams, 0, sizeof(destroyHandleParams));

  destroyHandleParams.dwVersion = NVFBC_DESTROY_HANDLE_PARAMS_VER;

  fbcStatus = pFn.nvFBCDestroyHandle(fbcHandle, &destroyHandleParams);
  if (fbcStatus != NVFBC_SUCCESS) {
    fprintf(stderr, "%s\n", pFn.nvFBCGetLastErrorStr(fbcHandle));
    return EXIT_FAILURE;
  }

  return EXIT_SUCCESS;
}
