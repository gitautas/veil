// It's not me, it's vulkan.
#![allow(clippy::too_many_arguments)]

use std::sync::{Arc, mpsc};
use std::time;

use anyhow::{Context, anyhow, bail};
use ash::{khr, vk};
use bytes::Bytes;
use tracing::{debug, error, instrument, trace, trace_span};

use self::gop_structure::HierarchicalP;
use crate::codec::VideoCodec;
use crate::session::control::VideoStreamParams;
use crate::vulkan::*;

mod dpb;
mod gop_structure;
mod rate_control;
mod stats;

mod h264;
use h264::H264Encoder;

mod h265;
use h265::H265Encoder;

mod av1;
use av1::Av1Encoder;

pub enum Encoder {
    H264(H264Encoder),
    H265(H265Encoder),
    Av1(Av1Encoder),
}

impl Encoder {
    pub fn new(
        vk: Arc<VkContext>,
        params: VideoStreamParams,
        framerate: u32,
        sink: impl Sink,
    ) -> anyhow::Result<Self> {
        match params.codec {
            VideoCodec::H264 => Ok(Self::H264(H264Encoder::new(vk, params, framerate, sink)?)),
            VideoCodec::H265 => Ok(Self::H265(H265Encoder::new(vk, params, framerate, sink)?)),
            VideoCodec::Av1 => Ok(Self::Av1(Av1Encoder::new(vk, params, framerate, sink)?)),
        }
    }

    pub unsafe fn submit_encode(
        &mut self,
        image: &VkImage,
        acquire: VkTimelinePoint,
        release: VkTimelinePoint,
    ) -> anyhow::Result<()> {
        match self {
            Self::H264(encoder) => unsafe { encoder.submit_encode(image, acquire, release) },
            Self::H265(encoder) => unsafe { encoder.submit_encode(image, acquire, release) },
            Self::Av1(encoder) => unsafe { encoder.submit_encode(image, acquire, release) },
        }
    }

    pub fn input_format(&self) -> vk::Format {
        match self {
            Self::H264(encoder) => encoder.input_format(),
            Self::H265(encoder) => encoder.input_format(),
            Self::Av1(encoder) => encoder.input_format(),
        }
    }

    pub fn create_input_image(&mut self) -> anyhow::Result<VkImage> {
        match self {
            Self::H264(encoder) => encoder.create_input_image(),
            Self::H265(encoder) => encoder.create_input_image(),
            Self::Av1(encoder) => encoder.create_input_image(),
        }
    }

    pub fn describe(&self) -> EncoderDescription {
        match self {
            Self::H264(encoder) => encoder.describe(),
            Self::H265(encoder) => encoder.describe(),
            Self::Av1(encoder) => encoder.describe(),
        }
    }

    /// Encoder statistics since the last call.
    pub fn take_stats(&self) -> crate::session::control::EncoderStats {
        match self {
            Self::H264(encoder) => encoder.stats().take_window(),
            Self::H265(encoder) => encoder.stats().take_window(),
            Self::Av1(encoder) => encoder.stats().take_window(),
        }
    }

    /// Whether input images can be written directly through per-plane storage
    /// views. If not, they're created with TRANSFER_DST and must be filled by
    /// copying.
    pub fn input_storage(&self) -> bool {
        match self {
            Self::H264(encoder) => encoder.input_storage(),
            Self::H265(encoder) => encoder.input_storage(),
            Self::Av1(encoder) => encoder.input_storage(),
        }
    }

    /// Cap the bitrate (e.g. at what the network can carry), or lift the
    /// cap with `None`. Takes effect on the next frame, without a keyframe.
    /// Only VBR presets have a bitrate to cap.
    pub fn set_bitrate_limit(&mut self, limit: Option<u64>) {
        match self {
            Self::H264(encoder) => encoder.set_bitrate_limit(limit),
            Self::H265(encoder) => encoder.set_bitrate_limit(limit),
            Self::Av1(encoder) => encoder.set_bitrate_limit(limit),
        }
    }

    /// (current target, the preset's target) average bitrates, for VBR.
    pub fn bitrates(&self) -> (Option<u64>, Option<u64>) {
        match self {
            Self::H264(encoder) => encoder.bitrates(),
            Self::H265(encoder) => encoder.bitrates(),
            Self::Av1(encoder) => encoder.bitrates(),
        }
    }

    pub fn request_refresh(&mut self) {
        match self {
            Encoder::H264(encoder) => encoder.request_refresh(),
            Encoder::H265(encoder) => encoder.request_refresh(),
            Encoder::Av1(encoder) => encoder.request_refresh(),
        }
    }
}

/// A rate control description for Vulkan: VkVideoEncodeRateControlInfoKHR,
/// its codec-specific extension `C`, and per-layer infos with their codec
/// extensions `L`, linked by pointer. Boxed so the pointers stay valid.
struct RcChain<C, L> {
    codec_layers: Vec<L>,
    layers: Vec<vk::VideoEncodeRateControlLayerInfoKHR<'static>>,
    codec: C,
    info: vk::VideoEncodeRateControlInfoKHR<'static>,
}

impl<C, L> RcChain<C, L> {
    /// `layers[i]` gets `codec_layers[i]` as its extension.
    fn new(
        mode: &rate_control::RateControlMode,
        codec_layers: Vec<L>,
        layers: Vec<vk::VideoEncodeRateControlLayerInfoKHR<'static>>,
        codec: C,
    ) -> Box<Self> {
        let vbv_size = match mode {
            rate_control::RateControlMode::Vbr(vbr) => vbr.vbv_size_ms,
            _ => 0,
        };
        let mut chain = Box::new(Self {
            codec_layers,
            layers,
            codec,
            info: vk::VideoEncodeRateControlInfoKHR::default()
                .rate_control_mode(mode.as_vk_flags())
                .virtual_buffer_size_in_ms(vbv_size),
        });
        let c = &mut *chain;
        for (layer, codec_layer) in c.layers.iter_mut().zip(c.codec_layers.iter_mut()) {
            layer.p_next = codec_layer as *mut L as *mut std::ffi::c_void;
        }
        c.info.layer_count = c.layers.len() as u32;
        c.info.p_layers = if c.layers.is_empty() { std::ptr::null() } else { c.layers.as_ptr() };
        c.info.p_next = &mut c.codec as *mut C as *mut std::ffi::c_void;
        chain
    }

    fn as_pnext(&mut self) -> *mut std::ffi::c_void {
        &mut self.info as *mut _ as *mut std::ffi::c_void
    }
}

/// How an encoder was configured, for display.
pub struct EncoderDescription {
    pub rate_control: String,
    pub temporal_layers: u32,
    pub input_storage: bool,
}

struct EncoderInner {
    session: vk::VideoSessionKHR,
    session_memory: Vec<vk::DeviceMemory>,

    session_params: vk::VideoSessionParametersKHR,

    writer_thread_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
    submitted_frames: Option<mpsc::Sender<EncoderOutputFrame>>,
    done_frames: mpsc::Receiver<EncoderOutputFrame>,

    dpb: dpb::DpbPool,

    width: u32,
    height: u32,
    framerate: u32,
    input_format: vk::Format,
    /// Whether input images support per-plane storage views.
    input_storage: bool,

    stats: stats::EncodeStats,

    vk: Arc<VkContext>,
}

impl EncoderInner {
    pub fn new(
        vk: Arc<VkContext>,
        width: u32,
        height: u32,
        framerate: u32,
        in_flight_frames: u32,
        required_dpb_size: usize,
        profile: &mut vk::VideoProfileInfoKHR,
        capabilities: vk::VideoCapabilitiesKHR,
        session_params_pnext: *mut std::ffi::c_void,
        sink: impl Sink,
    ) -> anyhow::Result<Self> {
        if vk.encode_queue.is_none() {
            bail!("no vulkan video support")
        }

        let video_apis = vk.video_apis.as_ref().unwrap();
        let video_instance = &video_apis.video_instance;
        let video_device = &video_apis.video_device;
        let encode_family = vk.device_info.encode_family.unwrap();

        if capabilities.max_coded_extent.width < width
            || capabilities.max_coded_extent.height < height
        {
            bail!(
                "video resolution too large: (max {}x{})",
                capabilities.max_coded_extent.width,
                capabilities.max_coded_extent.height
            );
        }

        let format_info = list_format_props(
            video_instance,
            vk.device_info.pdevice,
            profile,
            vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR,
        )?;

        for format in &format_info {
            trace!(?format, "available input format");
        }
        let input_format = match format_info.first() {
            Some(format) => format.format,
            None => bail!("unable to determine supported ENCODE_SRC format"),
        };

        // The convert shader can write straight into the encode image through
        // per-plane storage views, but only if the driver allows STORAGE plus
        // MUTABLE_FORMAT/EXTENDED_USAGE on encode source images (NVIDIA
        // doesn't). Otherwise the planes are written to separate images and
        // copied in, which only needs TRANSFER_DST.
        let input_storage = list_format_props(
            video_instance,
            vk.device_info.pdevice,
            profile,
            vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR | vk::ImageUsageFlags::STORAGE,
        )
        .map(|formats| {
            formats.iter().any(|f| {
                f.format == input_format
                    && f.image_tiling == vk::ImageTiling::OPTIMAL
                    && f.image_create_flags.contains(
                        vk::ImageCreateFlags::MUTABLE_FORMAT
                            | vk::ImageCreateFlags::EXTENDED_USAGE,
                    )
            })
        })
        .unwrap_or(false);

        trace!(?input_format, input_storage, width, height, "using input format");

        let buffer_size_alignment = capabilities.min_bitstream_buffer_size_alignment as usize;

        let session = {
            let create_info = vk::VideoSessionCreateInfoKHR::default()
                .queue_family_index(encode_family)
                .flags(vk::VideoSessionCreateFlagsKHR::ALLOW_ENCODE_PARAMETER_OPTIMIZATIONS)
                .video_profile(profile)
                .picture_format(input_format)
                .reference_picture_format(input_format)
                .max_coded_extent(capabilities.max_coded_extent)
                .max_dpb_slots(capabilities.max_dpb_slots)
                .max_active_reference_pictures(capabilities.max_active_reference_pictures)
                .std_header_version(&capabilities.std_header_version);

            unsafe {
                video_device
                    .create_video_session(&create_info, None)
                    .context("vkCreateVideoSessionKHR")?
            }
        };

        let session_memory =
            bind_session_memory(video_device, &vk.device, &vk.device_info, session)?;

        let session_params = {
            let mut create_info =
                vk::VideoSessionParametersCreateInfoKHR::default().video_session(session);
            create_info.p_next = session_params_pnext;

            unsafe {
                video_device
                    .create_video_session_parameters(&create_info, None)
                    .context("vkCreateVideoSessionParametersKHR")?
            }
        };

        let mut dpb = if capabilities
            .flags
            .contains(vk::VideoCapabilityFlagsKHR::SEPARATE_REFERENCE_IMAGES)
        {
            trace!("using separate images for DPB pool");

            dpb::DpbPool::new_separate_images(
                vk.clone(),
                input_format,
                width.next_multiple_of(capabilities.picture_access_granularity.width),
                height.next_multiple_of(capabilities.picture_access_granularity.height),
                profile,
                required_dpb_size,
            )?
        } else {
            trace!("using shared image for DPB pool");

            dpb::DpbPool::new(
                vk.clone(),
                input_format,
                width.next_multiple_of(capabilities.picture_access_granularity.width),
                height.next_multiple_of(capabilities.picture_access_granularity.height),
                profile,
                required_dpb_size,
            )?
        };

        // Without motion vector scaling, AV1 requires reference pictures to
        // have the same coded extent as the input. (The driver still pads
        // internally.)
        if profile.video_codec_operation == vk::VideoCodecOperationFlagsKHR::ENCODE_AV1 {
            dpb.set_coded_extent(vk::Extent2D { width, height });
        }

        let stats = stats::EncodeStats::default();

        let (submitted_frames_tx, submitted_frames_rx) = mpsc::channel();
        let (done_frames_tx, done_frames_rx) = mpsc::channel();

        let in_flight = in_flight_frames.max(1);
        for _frame in 0..in_flight {
            done_frames_tx
                .send(EncoderOutputFrame::new(
                    vk.clone(),
                    width,
                    height,
                    buffer_size_alignment,
                    profile,
                    #[cfg(feature = "tracy")]
                    tracy_client::frame_name!("encode"),
                )?)
                .unwrap();
        }

        let vk_clone = vk.clone();
        let stats_clone = stats.clone();
        let handle = std::thread::Builder::new()
            .name("encoder writer".to_owned())
            .spawn(move || {
                writer_thread(
                    vk_clone,
                    submitted_frames_rx,
                    done_frames_tx,
                    sink,
                    stats_clone,
                )
            })?;

        Ok(Self {
            session,
            session_params,
            session_memory,

            writer_thread_handle: Some(handle),
            submitted_frames: Some(submitted_frames_tx),
            done_frames: done_frames_rx,

            dpb,

            width,
            height,
            framerate,
            input_format,
            input_storage,

            stats,

            vk,
        })
    }

    fn create_input_image(&self, profile: &mut vk::VideoProfileInfoKHR) -> anyhow::Result<VkImage> {
        let image = {
            let mut profile_list_info = single_profile_list_info(profile);

            let (usage, flags) = if self.input_storage {
                (
                    vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR | vk::ImageUsageFlags::STORAGE,
                    vk::ImageCreateFlags::MUTABLE_FORMAT | vk::ImageCreateFlags::EXTENDED_USAGE,
                )
            } else {
                (
                    vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR | vk::ImageUsageFlags::TRANSFER_DST,
                    vk::ImageCreateFlags::empty(),
                )
            };

            let mut create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(self.input_format)
                .extent(vk::Extent3D {
                    width: self.width,
                    height: self.height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .flags(flags);
            create_info.p_next = &mut profile_list_info as *mut _ as *mut std::ffi::c_void;

            unsafe {
                self.vk
                    .device
                    .create_image(&create_info, None)
                    .context("VkCreateImage")?
            }
        };

        let memory = unsafe {
            bind_memory_for_image(&self.vk.device, &self.vk.device_info.memory_props, image)?
        };

        let view = unsafe {
            let mut usage_info = vk::ImageViewUsageCreateInfo::default()
                .usage(vk::ImageUsageFlags::VIDEO_ENCODE_SRC_KHR);

            let mut create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.input_format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            create_info.p_next = &mut usage_info as *mut _ as *mut std::ffi::c_void;

            self.vk
                .device
                .create_image_view(&create_info, None)
                .context("VkCreateImageView")?
        };

        Ok(VkImage::wrap(
            self.vk.clone(),
            image,
            view,
            memory,
            self.input_format,
            self.width,
            self.height,
        ))
    }

    #[instrument(skip_all)]
    pub unsafe fn submit_encode(
        &mut self,
        input: &VkImage,
        tp_acquire: VkTimelinePoint,
        tp_release: VkTimelinePoint,
        frame_state: &gop_structure::GopFrame,
        rc_info_pnext: *mut std::ffi::c_void,
        rc_update_pnext: Option<*mut std::ffi::c_void>,
        codec_pic_info_pnext: *mut std::ffi::c_void,
        codec_setup_info_pnext: *mut std::ffi::c_void,
        codec_ref_info_pnext: &mut [*mut std::ffi::c_void],
        insert: Option<Bytes>,
    ) -> anyhow::Result<()> {
        use ash::vk::Handle;

        debug!(
            "EncoderInner::submit_encode entry: gop_pos={} is_keyframe={}",
            frame_state.gop_position, frame_state.is_keyframe
        );

        if self.session_params.is_null() {
            bail!("session parameters not yet created");
        }

        let video_apis = self.vk.video_apis.as_ref().unwrap();
        let video_device = &video_apis.video_device;
        let encode_loader = &video_apis.encode_device;
        let encode_queue = self.vk.encode_queue.as_ref().unwrap();

        // "Acquire" a buffer to copy to. This provides backpressure if the
        // encoder can't keep up.
        let res = trace_span!("wait_prev_frame").in_scope(|| self.done_frames.recv());
        let mut frame = match res {
            Ok(frame) => frame,
            Err(_) => {
                bail!("copy thread died");
            }
        };
        frame.submitted_at = time::Instant::now();

        #[cfg(feature = "tracy")]
        {
            frame.tracy_context.frame = Some(
                tracy_client::Client::running()
                    .expect("no tracy client")
                    .non_continuous_frame(frame.tracy_context.frame_name),
            );

            if let Some(ref ctx) = encode_queue.tracy_context {
                frame.tracy_context.encode_span =
                    Some(ctx.span(tracy_client::span_location!("encode"))?);
            }
        }

        unsafe {
            begin_command_buffer(&self.vk.device, frame.encode_cb)?;
        }

        // Record the start timestamp.
        #[cfg(feature = "tracy")]
        if let Some(encode_ts_pool) = &mut frame.tracy_context.encode_ts_pool {
            unsafe {
                encode_ts_pool.cmd_reset(&self.vk.device, frame.encode_cb);
                self.vk.device.cmd_write_timestamp(
                    frame.encode_cb,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    encode_ts_pool.pool,
                    0,
                );
            }
        }

        // Acquire the image from the graphics queue.
        insert_image_barrier(
            &self.vk.device,
            frame.encode_cb,
            input.image,
            Some((self.vk.graphics_queue.family, encode_queue.family)),
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::VIDEO_ENCODE_SRC_KHR,
            vk::PipelineStageFlags2::NONE,
            vk::AccessFlags2::NONE,
            vk::PipelineStageFlags2::VIDEO_ENCODE_KHR,
            vk::AccessFlags2::VIDEO_ENCODE_READ_KHR,
        );

        // Bind the setup picture and any reference pictures.
        let setup_pic = self.dpb.setup_pic();
        let ref_pics = frame_state
            .ref_ids
            .iter()
            .map(|id| {
                self.dpb
                    .get_pic(*id)
                    .ok_or(anyhow!("ref pic {id} missing from dpb"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut bound_pics = vec![
            vk::VideoReferenceSlotInfoKHR::default()
                .slot_index(if setup_pic.currently_active {
                    setup_pic.index as i32
                } else {
                    -1
                })
                .picture_resource(&setup_pic.picture_resource_info),
        ];

        for ref_pic in &ref_pics {
            assert!(ref_pic.currently_active);
            bound_pics.push(
                vk::VideoReferenceSlotInfoKHR::default()
                    .slot_index(ref_pic.index as i32)
                    .picture_resource(&ref_pic.picture_resource_info),
            );
        }

        trace!(
            ref_ids = ?frame_state.ref_ids,
            ref_slots = ?ref_pics.iter().map(|p| p.index).collect::<Vec<_>>(),
            setup_id = frame_state.id,
            setup_slot = setup_pic.index,
            gop_position = frame_state.gop_position,
            is_keyframe = frame_state.is_keyframe,
            forward_ref_count = frame_state.forward_ref_count,
            input_image = ?input.image,
            "encoding frame"
        );

        // Bind the session.
        {
            let mut begin_info = vk::VideoBeginCodingInfoKHR::default()
                .flags(vk::VideoBeginCodingFlagsKHR::empty())
                .video_session(self.session)
                .video_session_parameters(self.session_params)
                .reference_slots(&bound_pics);

            // Vulkan wants us to inform it of the current rate control, which
            // is unset on the first frame. A change (`rc_update_pnext`) only
            // takes effect with the control command below.
            if frame_state.stream_position != 0 {
                begin_info.p_next = rc_info_pnext;
            }

            unsafe {
                video_device.cmd_begin_video_coding(frame.encode_cb, &begin_info);
            };
        }

        // Reset on keyframes, and apply rate control changes (e.g. a new
        // bitrate limit) whenever they come.
        if frame_state.is_keyframe || rc_update_pnext.is_some() {
            let flags = if frame_state.is_keyframe {
                vk::VideoCodingControlFlagsKHR::RESET
                    | vk::VideoCodingControlFlagsKHR::ENCODE_RATE_CONTROL
            } else {
                vk::VideoCodingControlFlagsKHR::ENCODE_RATE_CONTROL
            };
            let mut ctrl_info = vk::VideoCodingControlInfoKHR::default().flags(flags);
            ctrl_info.p_next = rc_update_pnext.unwrap_or(rc_info_pnext);

            unsafe {
                video_device.cmd_control_video_coding(frame.encode_cb, &ctrl_info);
            };
        }

        // Encode.
        unsafe {
            self.vk.device.cmd_begin_query(
                frame.encode_cb,
                frame.query_pool,
                0,
                vk::QueryControlFlags::empty(),
            );
        }

        {
            // The input picture.
            let src_pic_resource = vk::VideoPictureResourceInfoKHR::default()
                .coded_extent(vk::Extent2D {
                    width: self.width,
                    height: self.height,
                })
                .image_view_binding(input.view);

            // The slot we're writing to.
            let mut setup_reference_slot = vk::VideoReferenceSlotInfoKHR::default()
                .slot_index(setup_pic.index as i32)
                .picture_resource(&setup_pic.picture_resource_info);
            setup_reference_slot.p_next = codec_setup_info_pnext;

            // The slots we're referencing.
            let reference_slots = ref_pics
                .iter()
                .zip(codec_ref_info_pnext.iter_mut())
                .map(|(ref_pic, codec_ref_pnext)| {
                    let mut slot = vk::VideoReferenceSlotInfoKHR::default()
                        .slot_index(ref_pic.index as i32)
                        .picture_resource(&ref_pic.picture_resource_info);
                    slot.p_next = *codec_ref_pnext;
                    slot
                })
                .collect::<Vec<_>>();

            let mut encode_info = vk::VideoEncodeInfoKHR::default()
                .flags(vk::VideoEncodeFlagsKHR::empty())
                .dst_buffer(frame.copy_buffer.buffer)
                .dst_buffer_range(frame.copy_buffer.len as u64)
                .src_picture_resource(src_pic_resource)
                .setup_reference_slot(&setup_reference_slot)
                .reference_slots(&reference_slots);
            encode_info.p_next = codec_pic_info_pnext;

            // Transition the DPB images/layers we need.
            let mut dpb_barriers = Vec::new();
            for pic in &ref_pics {
                dpb_barriers.push(
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::NONE)
                        .src_access_mask(vk::AccessFlags2::NONE)
                        .dst_stage_mask(vk::PipelineStageFlags2::VIDEO_ENCODE_KHR)
                        .dst_access_mask(vk::AccessFlags2::VIDEO_ENCODE_READ_KHR)
                        .old_layout(vk::ImageLayout::VIDEO_ENCODE_DPB_KHR)
                        .new_layout(vk::ImageLayout::VIDEO_ENCODE_DPB_KHR)
                        .image(pic.image)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: vk::REMAINING_MIP_LEVELS,
                            // For multiple-layers-in-one-image DPB, just the layer referenced.
                            base_array_layer: pic.picture_resource_info.base_array_layer,
                            layer_count: 1,
                        }),
                );
            }

            dpb_barriers.push(
                vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::NONE)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_stage_mask(vk::PipelineStageFlags2::VIDEO_ENCODE_KHR)
                    .dst_access_mask(
                        vk::AccessFlags2::VIDEO_ENCODE_WRITE_KHR
                            | vk::AccessFlags2::VIDEO_ENCODE_READ_KHR,
                    )
                    .old_layout(vk::ImageLayout::VIDEO_ENCODE_DPB_KHR)
                    .new_layout(vk::ImageLayout::VIDEO_ENCODE_DPB_KHR)
                    .image(setup_pic.image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: vk::REMAINING_MIP_LEVELS,
                        base_array_layer: setup_pic.picture_resource_info.base_array_layer,
                        layer_count: 1,
                    }),
            );

            unsafe {
                self.vk.device.cmd_pipeline_barrier2(
                    frame.encode_cb,
                    &vk::DependencyInfo::default().image_memory_barriers(&dpb_barriers),
                );
            }

            // Update state as if the operation succeeded.
            if frame_state.forward_ref_count > 0 {
                // Keyframes clear all dpb slots.
                if frame_state.is_keyframe {
                    self.dpb.clear();
                }

                self.dpb.mark_active(setup_pic.index, frame_state.id);
            } else {
                self.dpb.mark_inactive(setup_pic.index);
            }

            unsafe {
                encode_loader.cmd_encode_video(frame.encode_cb, &encode_info);
            };
        }

        unsafe {
            self.vk
                .device
                .cmd_end_query(frame.encode_cb, frame.query_pool, 0);
        }

        // Unbind the session.
        {
            let end_info =
                vk::VideoEndCodingInfoKHR::default().flags(vk::VideoEndCodingFlagsKHR::empty());

            unsafe {
                video_device.cmd_end_video_coding(frame.encode_cb, &end_info);
            };
        }

        // Release the input picture back to the graphics queue.
        insert_image_barrier(
            &self.vk.device,
            frame.encode_cb,
            input.image,
            Some((encode_queue.family, self.vk.graphics_queue.family)),
            vk::ImageLayout::VIDEO_ENCODE_SRC_KHR,
            vk::ImageLayout::GENERAL,
            vk::PipelineStageFlags2::VIDEO_ENCODE_KHR,
            vk::AccessFlags2::VIDEO_ENCODE_WRITE_KHR,
            vk::PipelineStageFlags2::empty(),
            vk::AccessFlags2::empty(),
        );

        // Record the end timestamp.
        #[cfg(feature = "tracy")]
        if let Some(encode_ts_pool) = &mut frame.tracy_context.encode_ts_pool {
            unsafe {
                self.vk.device.cmd_write_timestamp(
                    frame.encode_cb,
                    vk::PipelineStageFlags::ALL_COMMANDS,
                    encode_ts_pool.pool,
                    1,
                );
            }
        }

        #[cfg(feature = "tracy")]
        if let Some(span) = &mut frame.tracy_context.encode_span {
            span.end_zone();
        }

        // Wait for the output buffer to be clear of the previous copy
        // operation, then establish new timeline points.
        unsafe {
            frame.tp_copied.wait()?;
        }
        frame.tp_encoded += 10;
        frame.tp_copied = &frame.tp_encoded + 1;

        // Submit!
        {
            unsafe { self.vk.device.end_command_buffer(frame.encode_cb)? };

            let cb_infos = [vk::CommandBufferSubmitInfo::default().command_buffer(frame.encode_cb)];

            let wait_infos = [vk::SemaphoreSubmitInfo::default()
                .semaphore(tp_acquire.timeline().as_semaphore())
                .value(tp_acquire.into())
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];

            let signal_infos = [
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(frame.timeline.as_semaphore())
                    .value(frame.tp_encoded.value())
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(tp_release.timeline().as_semaphore())
                    .value(tp_release.value())
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
            ];

            let submit_info = vk::SubmitInfo2::default()
                .wait_semaphore_infos(&wait_infos)
                .signal_semaphore_infos(&signal_infos)
                .command_buffer_infos(&cb_infos);

            let encode_queue = self.vk.encode_queue.as_ref().unwrap();
            unsafe {
                self.vk
                    .device
                    .queue_submit2(encode_queue.queue, &[submit_info], vk::Fence::null())
                    .context("vkQueueSubmit")?;
            }

            debug!(
                "EncoderInner::submit_encode: queue_submit2 completed, tp_encoded={}",
                frame.tp_encoded.value()
            );
        }

        frame.hierarchical_layer = frame_state.id;
        frame.is_keyframe = frame_state.is_keyframe;
        if let Some(submitted_frames) = &self.submitted_frames {
            // Tell the other thread to copy out the finished packet when it's
            // finished. Optionally insert headers.
            frame.headers = insert;

            debug!(
                "EncoderInner::submit_encode: sending frame to writer thread, is_keyframe={}",
                frame.is_keyframe
            );

            submitted_frames
                .send(frame)
                .map_err(|_| anyhow::anyhow!("writer thread died"))?;

            debug!("EncoderInner::submit_encode: frame sent to writer thread successfully");
        } else {
            debug!("EncoderInner::submit_encode: no writer thread (submitted_frames is None)");
        }

        Ok(())
    }
}

impl Drop for EncoderInner {
    fn drop(&mut self) {
        drop(self.submitted_frames.take());
        for done in self.done_frames.iter() {
            drop(done)
        }

        if let Some(handle) = self.writer_thread_handle.take() {
            match handle.join() {
                Ok(Ok(())) => (),
                Ok(Err(e)) => error!("copy thread exited with error: {:#}", e),
                Err(_) => error!("copy thread panicked"),
            }
        }

        debug!("stream stats: \n{:#?}", self.stats);

        let video_apis = self.vk.video_apis.as_ref().unwrap();
        let video_device = &video_apis.video_device;

        unsafe {
            let _ = self.vk.device.device_wait_idle();

            video_device.destroy_video_session(self.session, None);
            video_device.destroy_video_session_parameters(self.session_params, None);

            for memory in self.session_memory.drain(..) {
                self.vk.device.free_memory(memory, None);
            }
        }
    }
}

/// A synchronized buffer for writing encoded frames to. Passed back and forth
/// between the submission thread and the copy thread.
struct EncoderOutputFrame {
    encode_cb: vk::CommandBuffer,
    copy_buffer: VkHostBuffer,
    query_pool: vk::QueryPool,

    hierarchical_layer: u32,
    is_keyframe: bool,
    headers: Option<bytes::Bytes>,
    /// When the frame was submitted to the encoder, for stats.
    submitted_at: time::Instant,

    timeline: VkTimelineSemaphore,
    tp_encoded: VkTimelinePoint,
    tp_copied: VkTimelinePoint,

    #[cfg(feature = "tracy")]
    tracy_context: TracingContext,

    vk: Arc<VkContext>,
}

#[cfg(feature = "tracy")]
struct TracingContext {
    frame_name: tracy_client::FrameName,
    frame: Option<tracy_client::Frame>,
    encode_span: Option<tracy_client::GpuSpan>,
    encode_ts_pool: Option<VkTimestampQueryPool>,
}

impl EncoderOutputFrame {
    pub fn new(
        vk: Arc<VkContext>,
        width: u32,
        height: u32,
        buffer_size_alignment: usize,
        profile: &mut vk::VideoProfileInfoKHR,
        #[cfg(feature = "tracy")] frame_name: tracy_client::FrameName,
    ) -> anyhow::Result<Self> {
        let buffer_size = (width * height * 3).next_multiple_of(buffer_size_alignment as u32);

        let mut profile_list_info = single_profile_list_info(profile);

        let copy_buffer = {
            let buf = {
                let mut create_info = vk::BufferCreateInfo::default()
                    .size(buffer_size as u64)
                    .usage(vk::BufferUsageFlags::VIDEO_ENCODE_DST_KHR)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE);
                create_info.p_next = &mut profile_list_info as *mut _ as *mut std::ffi::c_void;

                unsafe { vk.device.create_buffer(&create_info, None)? }
            };

            let requirements = unsafe { vk.device.get_buffer_memory_requirements(buf) };

            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(vk.device_info.host_visible_mem_type_index);

            let memory = unsafe { vk.device.allocate_memory(&alloc_info, None)? };

            unsafe {
                vk.device
                    .bind_buffer_memory(buf, memory, 0)
                    .context("vkBindBufferMemory")?
            };

            VkHostBuffer::wrap(vk.clone(), buf, memory, buffer_size as usize)
        };

        let encode_queue = vk.encode_queue.as_ref().unwrap();
        let encode_cb = allocate_command_buffer(&vk.device, encode_queue.command_pool)?;

        let query_pool = {
            let mut video_pool_info = vk::QueryPoolVideoEncodeFeedbackCreateInfoKHR::default()
                .encode_feedback_flags(
                    vk::VideoEncodeFeedbackFlagsKHR::BITSTREAM_BUFFER_OFFSET
                        | vk::VideoEncodeFeedbackFlagsKHR::BITSTREAM_BYTES_WRITTEN,
                );

            let mut create_info = vk::QueryPoolCreateInfo::default()
                .query_type(vk::QueryType::VIDEO_ENCODE_FEEDBACK_KHR)
                .query_count(1);
            // The spec requires VkVideoProfileInfoKHR's pNext chain to include the
            // codec-specific profile struct (e.g. VkVideoEncodeH264ProfileInfoKHR).
            // Preserve that chain by inserting the feedback-info struct between
            // profile_copy and its original pNext, giving:
            //   QueryPoolCreateInfo → VideoProfileInfoKHR
            //     → QueryPoolVideoEncodeFeedbackCreateInfoKHR
            //       → VideoEncodeUsageInfoKHR → VkVideoEncodeH264ProfileInfoKHR
            let mut profile_copy = *profile;
            video_pool_info.p_next = profile_copy.p_next; // forward to codec profile chain
            profile_copy.p_next = &mut video_pool_info as *mut _ as *mut std::ffi::c_void;
            create_info.p_next = &mut profile_copy as *mut _ as *mut std::ffi::c_void;

            unsafe {
                let query_pool = vk
                    .device
                    .create_query_pool(&create_info, None)
                    .context("vkCreateQueryPool")?;
                vk.device.reset_query_pool(query_pool, 0, 1);

                query_pool
            }
        };

        let timeline = VkTimelineSemaphore::new(vk.clone(), 0)?;

        #[cfg(feature = "tracy")]
        let encode_ts_pool = if matches!(
            vk.device_info.driver_version,
            DriverVersion::MesaRadv { .. }
        ) {
            // RADV offers support for timestamp queries, but then has an
            // assertion at timestamp write time.
            None
        } else {
            create_timestamp_query_pool(&vk.device, 2).ok()
        };

        Ok(EncoderOutputFrame {
            encode_cb,
            copy_buffer,
            query_pool,

            hierarchical_layer: 0,
            is_keyframe: false,
            headers: None,

            submitted_at: time::Instant::now(),
            tp_encoded: timeline.new_point(0),
            tp_copied: timeline.new_point(0),
            timeline,

            #[cfg(feature = "tracy")]
            tracy_context: TracingContext {
                frame_name,
                frame: None,
                encode_span: None,
                encode_ts_pool,
            },

            vk,
        })
    }
}

impl Drop for EncoderOutputFrame {
    fn drop(&mut self) {
        unsafe {
            let device = &self.vk.device;
            let encode_queue = self.vk.encode_queue.as_ref().unwrap();

            device.queue_wait_idle(encode_queue.queue).unwrap();
            device.free_command_buffers(encode_queue.command_pool, &[self.encode_cb]);
            device.destroy_query_pool(self.query_pool, None);

            #[cfg(feature = "tracy")]
            if let Some(pool) = self.tracy_context.encode_ts_pool.take() {
                device.destroy_query_pool(pool.pool, None);
            }
        }
    }
}

// SAFETY: the contained pointers are nothing fancy.
unsafe impl Send for EncoderOutputFrame {}

/// Allows the caller to decide where to sink the frames.
pub trait Sink: Send + 'static {
    fn write_frame(
        &mut self,
        ts: time::Instant,
        frame: Bytes,
        hierarchical_layer: u32,
        is_keyframe: bool,
    );
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct QueryResults {
    offset: i32,
    size: i32,
    result: i32,
}

/// Responsible for copying encoded frames from the output buffer and
/// dispatching them to the client. Passes instances of `EncodedOutputFrame`
/// back and forth with the main thread.
fn writer_thread(
    vk: Arc<VkContext>,
    input: mpsc::Receiver<EncoderOutputFrame>,
    done: mpsc::Sender<EncoderOutputFrame>,
    mut sink: impl Sink,
    stats: stats::EncodeStats,
) -> anyhow::Result<()> {
    use tracing::info;

    let device = &vk.device;

    let mut capture_ts = time::Instant::now();
    let mut frame_count = 0u64;

    info!("writer_thread: started, waiting for frames");

    for mut frame in input {
        frame_count += 1;
        if frame_count % 60 == 0 {
            trace!("writer_thread: received frame {}", frame_count);
        }
        let dur = capture_ts.elapsed();
        capture_ts = time::Instant::now();

        // Wait for the frame to finish encoding.
        unsafe {
            frame.tp_encoded.wait()?;
        }
        stats.record_encode_time(frame.submitted_at.elapsed());

        #[cfg(feature = "tracy")]
        {
            frame.tracy_context.frame.take();
            if let Some(span) = frame.tracy_context.encode_span.take() {
                if let Some(pool) = &mut frame.tracy_context.encode_ts_pool {
                    let timestamps = pool.fetch_results(device)?;
                    span.upload_timestamp_start(timestamps[0]);
                    span.upload_timestamp_end(timestamps[1]);
                }
            }
        }

        // Get the buffer offsets for the encoded data.
        let mut results = [QueryResults::default()];
        unsafe {
            device
                .get_query_pool_results(
                    frame.query_pool,
                    0,
                    &mut results,
                    vk::QueryResultFlags::WITH_STATUS_KHR,
                )
                .context("vkGetQueryPoolResults")?;
            device.reset_query_pool(frame.query_pool, 0, 1)
        }

        let res = vk::QueryResultStatusKHR::from_raw(results[0].result);
        if res != vk::QueryResultStatusKHR::COMPLETE {
            bail!("encode failed: {:?}", res);
        }

        if frame_count % 60 == 0 {
            trace!(
                "writer_thread: frame {} encoded, offset={} size={} status={:?}",
                frame_count, results[0].offset, results[0].size, res
            );
        }

        trace!(len = results[0].size, ?dur, "encoded packet");
        stats.record_frame_size(
            frame.is_keyframe,
            frame.hierarchical_layer,
            results[0].size as usize,
        );

        let data = unsafe {
            let ptr = frame.copy_buffer.access as *const u8;
            std::slice::from_raw_parts(
                ptr.add(results[0].offset as usize),
                results[0].size as usize,
            )
        };

        // Prepend any headers.
        let data = if let Some(headers) = frame.headers.take() {
            let mut buf = bytes::BytesMut::from(headers);
            buf.extend_from_slice(data);
            buf.freeze()
        } else {
            Bytes::copy_from_slice(data)
        };

        unsafe {
            frame.tp_copied.signal()?;
        }

        sink.write_frame(
            capture_ts,
            data,
            frame.hierarchical_layer,
            frame.is_keyframe,
        );

        if frame_count % 60 == 0 {
            trace!("writer_thread: frame {} written to sink", frame_count);
        }

        done.send(frame).ok();
    }

    info!(
        "writer_thread: exiting, processed {} frames total",
        frame_count
    );
    Ok(())
}

fn list_format_props<'a>(
    video_instance: &'a khr::video_queue::Instance,
    pdevice: vk::PhysicalDevice,
    profile: &mut vk::VideoProfileInfoKHR,
    usage: vk::ImageUsageFlags,
) -> anyhow::Result<Vec<vk::VideoFormatPropertiesKHR<'a>>> {
    let mut profile_list_info = single_profile_list_info(profile);
    let mut format_info = vk::PhysicalDeviceVideoFormatInfoKHR::default().image_usage(usage);
    format_info.p_next = &mut profile_list_info as *mut _ as *mut std::ffi::c_void;

    unsafe {
        let count = video_instance
            .get_physical_device_video_format_properties_len(pdevice, &format_info)
            .context("vkGetVideoFormatPropertiesKHR (count)")?;

        let mut props = vec![vk::VideoFormatPropertiesKHR::default(); count];
        video_instance
            .get_physical_device_video_format_properties(pdevice, &format_info, &mut props)
            .context("vkGetVideoFormatPropertiesKHR")?;

        Ok(props)
    }
}

fn bind_session_memory(
    video_device: &khr::video_queue::Device,
    device: &ash::Device,
    device_info: &VkDeviceInfo,
    session: vk::VideoSessionKHR,
) -> anyhow::Result<Vec<vk::DeviceMemory>> {
    let mut session_memory = Vec::new();
    let reqs = unsafe {
        let count = video_device.get_video_session_memory_requirements_len(session)?;
        let mut reqs = vec![vk::VideoSessionMemoryRequirementsKHR::default(); count];
        video_device.get_video_session_memory_requirements(session, &mut reqs)?;
        reqs
    };

    let mut binds = Vec::new();
    for req in reqs.into_iter() {
        let memory = {
            let mut alloc_info =
                vk::MemoryAllocateInfo::default().allocation_size(req.memory_requirements.size);

            let mem_type_idx = select_memory_type(
                &device_info.memory_props,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                Some(req.memory_requirements.memory_type_bits),
            )
            .or_else(|| {
                select_memory_type(
                    &device_info.memory_props,
                    vk::MemoryPropertyFlags::empty(),
                    Some(req.memory_requirements.memory_type_bits),
                )
            });

            if mem_type_idx.is_none() {
                bail!("no suitable memory type for video session");
            }

            alloc_info = alloc_info.memory_type_index(mem_type_idx.unwrap());

            unsafe {
                device
                    .allocate_memory(&alloc_info, None)
                    .context("vkAllocateMemory")?
            }
        };

        session_memory.push(memory);
        binds.push(
            vk::BindVideoSessionMemoryInfoKHR::default()
                .memory_bind_index(req.memory_bind_index)
                .memory(memory)
                .memory_size(req.memory_requirements.size),
        );
    }

    unsafe {
        video_device
            .bind_video_session_memory(session, &binds)
            .context("vkBindVideoSessionMemory")?
    }

    Ok(session_memory)
}

fn default_profile(op: vk::VideoCodecOperationFlagsKHR) -> vk::VideoProfileInfoKHR<'static> {
    vk::VideoProfileInfoKHR::default()
        .video_codec_operation(op)
        .chroma_subsampling(vk::VideoChromaSubsamplingFlagsKHR::TYPE_420)
        .chroma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8)
        .luma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8)
}

fn default_hdr10_profile(op: vk::VideoCodecOperationFlagsKHR) -> vk::VideoProfileInfoKHR<'static> {
    vk::VideoProfileInfoKHR::default()
        .video_codec_operation(op)
        .chroma_subsampling(vk::VideoChromaSubsamplingFlagsKHR::TYPE_420)
        .chroma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_10)
        .luma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_10)
}

fn default_encode_usage(driver_version: DriverVersion) -> vk::VideoEncodeUsageInfoKHR<'static> {
    // Nvidia chokes on "ULTRA LOW" for some reason.
    let tuning_mode = if matches!(driver_version, DriverVersion::NvidiaProprietary { .. }) {
        vk::VideoEncodeTuningModeKHR::LOW_LATENCY
    } else {
        vk::VideoEncodeTuningModeKHR::ULTRA_LOW_LATENCY
    };

    vk::VideoEncodeUsageInfoKHR::default()
        .video_usage_hints(vk::VideoEncodeUsageFlagsKHR::STREAMING)
        .video_content_hints(vk::VideoEncodeContentFlagsKHR::RENDERED)
        .tuning_mode(tuning_mode)
}

fn single_profile_list_info<'a>(
    profile: &'a vk::VideoProfileInfoKHR,
) -> vk::VideoProfileListInfoKHR<'a> {
    vk::VideoProfileListInfoKHR {
        p_profiles: profile as *const _,
        profile_count: 1,
        ..Default::default()
    }
}

fn default_structure(
    codec: VideoCodec,
    max_codec_layers: u32,
    max_dpb_slots: u32,
    gop_size: u32,
) -> anyhow::Result<HierarchicalP> {
    const MAX_LAYERS: u32 = 4;
    const DEFAULT_GOP_SIZE: u32 = 60;

    let effective_gop = if gop_size == 0 { DEFAULT_GOP_SIZE } else { gop_size };

    // Disable hierarchical coding on H264, because it's broken.
    let mut layers = if codec == VideoCodec::H264 {
        1
    } else {
        std::cmp::min(MAX_LAYERS, max_codec_layers).max(1)
    };

    // The GOP has to be a whole number of mini-GOPs (2^(layers-1) frames), so
    // use fewer layers rather than changing the requested keyframe interval.
    while effective_gop % 2_u32.pow(layers - 1) != 0 {
        layers -= 1;
    }

    let mut structure = HierarchicalP::new(layers, effective_gop);
    while structure.required_dpb_size() as u32 > max_dpb_slots {
        layers -= 1;
        if layers == 0 {
            bail!("max_dpb_slots too low");
        }

        structure = HierarchicalP::new(layers, effective_gop);
    }

    Ok(structure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_structure_fits_gop_size() {
        // RADV reports 4 H.265 sub-layers; a 30-frame GOP only fits 2.
        let structure = default_structure(VideoCodec::H265, 4, 17, 30).unwrap();
        assert_eq!(structure.layers, 2);
        assert_eq!(structure.gop_size, 30);

        let structure = default_structure(VideoCodec::H265, 4, 17, 64).unwrap();
        assert_eq!(structure.layers, 4);

        let structure = default_structure(VideoCodec::H265, 4, 17, 0).unwrap();
        assert_eq!(structure.layers, 3);
        assert_eq!(structure.gop_size, 60);
    }
}
