# veil-sigrun
NV based screen capture and encoding tool. Only for local testing on NV systems.
After doing more research, this needs to be re-done. The way to go about this is by using DRM to get acces to a dmabuf of the GPU framebuffer, which would allow this to work with both NV and AMD GPUs, all that would be needed is to write implementations for NvENC and AMF to use the hardware encoders. 
