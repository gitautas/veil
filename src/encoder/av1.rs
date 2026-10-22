//! Experimental AV1 encoding (VK_KHR_video_encode_av1).
//!
//! Deliberately simple: a flat P structure (every frame references the one
//! before it), no temporal layers, and every frame refreshes all eight AV1
//! reference slots, so the references never need tracking.

use std::mem::MaybeUninit;
use std::sync::Arc;

use anyhow::{Context, bail};
use ash::vk;
use bytes::{Bytes, BytesMut};
use tracing::{debug, trace, warn};

use super::gop_structure::HierarchicalP;
use super::rate_control::{self, RateControlMode};
use crate::codec::VideoCodec;
use crate::color::VideoProfile;
use crate::{session::control::VideoStreamParams, vulkan::*};

vk_chain! {
    pub struct Av1EncodeProfile<'a> {
        pub profile_info: vk::VideoProfileInfoKHR<'a>,
        pub encode_usage_info: vk::VideoEncodeUsageInfoKHR<'a>,
        pub av1_profile: vk::VideoEncodeAV1ProfileInfoKHR<'a>,
    }
}

vk_chain! {
    pub struct Av1EncodeCapabilities<'a> {
        pub video_caps: vk::VideoCapabilitiesKHR<'a>,
        pub encode_caps: vk::VideoEncodeCapabilitiesKHR<'a>,
        pub av1_caps: vk::VideoEncodeAV1CapabilitiesKHR<'a>,
    }
}

/// A temporal delimiter OBU, which starts every temporal unit.
const TEMPORAL_DELIMITER: [u8; 2] = [0x12, 0x00];

/// "No primary reference frame": each frame starts from default CDFs.
const PRIMARY_REF_NONE: u8 = 7;

/// The highest level we'll signal (5.1: up to 4K at 60 fps).
const MAX_LEVEL: vk::native::StdVideoAV1Level = vk::native::StdVideoAV1Level_STD_VIDEO_AV1_LEVEL_5_1;

/// The encoder's rate control works in H.26x-style QP (0-51); AV1 uses a
/// quantizer index (0-255).
fn qp_to_q_index(qp: u32) -> u32 {
    (qp * 5).min(255)
}

/// The number of bits needed to code `n`, minus one, as the sequence header
/// wants for the frame dimensions.
fn bits_minus_1(n: u32) -> u8 {
    (u32::BITS - n.max(1).leading_zeros() - 1) as u8
}

#[derive(Debug, Default, Clone, Copy)]
struct Av1Metadata {
    frame_type: vk::native::StdVideoAV1FrameType,
    order_hint: u8,
}

pub struct Av1Encoder {
    inner: super::EncoderInner,
    profile: Av1EncodeProfile,
    rc_mode: RateControlMode,
    min_q_index: u32,
    max_q_index: u32,

    structure: HierarchicalP,
    /// The picture in the (single) reference slot.
    last: Av1Metadata,
    /// The real picture size, signalled as the render size, since the driver
    /// pads the coded frame (e.g. to 1088 rows).
    render_size: (u16, u16),
    order_hint: u8,

    /// Temporal delimiter + sequence header, sent with each keyframe.
    keyframe_headers: Bytes,
}

impl Av1Encoder {
    pub fn new(
        vk: Arc<VkContext>,
        params: VideoStreamParams,
        framerate: u32,
        sink: impl super::Sink,
    ) -> anyhow::Result<Self> {
        let video_apis = vk.video_apis.as_ref().unwrap();
        let video_instance = &video_apis.video_instance;
        let encode_device = &video_apis.encode_device;

        if !vk.device_info.supports_av1 {
            bail!("this gpu can't encode AV1");
        }
        if params.profile == VideoProfile::Hdr10 {
            bail!("AV1 encoding is 8-bit only for now");
        }

        let op = vk::VideoCodecOperationFlagsKHR::ENCODE_AV1;
        let av1_profile_info = vk::VideoEncodeAV1ProfileInfoKHR::default()
            .std_profile(vk::native::StdVideoAV1Profile_STD_VIDEO_AV1_PROFILE_MAIN);

        let mut profile = Av1EncodeProfile::new(
            super::default_profile(op),
            super::default_encode_usage(vk.device_info.driver_version.clone()),
            av1_profile_info,
        );

        let mut caps = Av1EncodeCapabilities::default();
        unsafe {
            video_instance
                .get_physical_device_video_capabilities(
                    vk.device_info.pdevice,
                    &profile.profile_info,
                    caps.as_mut(),
                )
                .context("vkGetPhysicalDeviceVideoCapabilitiesKHR")?;
        };

        trace!("video capabilities: {:#?}", caps.video_caps);
        trace!("encode capabilities: {:#?}", caps.encode_caps);
        trace!("av1 capabilities: {:#?}", caps.av1_caps);

        if caps.av1_caps.single_reference_name_mask & 1 == 0 {
            bail!("AV1 encoder can't reference LAST_FRAME");
        }
        if caps.av1_caps.requires_gop_remaining_frames == vk::TRUE {
            // We'd need VkVideoEncodeAV1GopRemainingFrameInfoKHR on every frame.
            bail!("AV1 encoder requires GOP remaining-frame info, which isn't implemented");
        }

        // No temporal layers: one reference, the previous frame.
        let structure = super::default_structure(
            VideoCodec::Av1,
            1,
            caps.video_caps.max_dpb_slots,
            params.gop_size,
        )?;

        let min_q_index = caps.av1_caps.min_q_index;
        let max_q_index = caps.av1_caps.max_q_index.min(255);
        let rc_mode = rate_control::select_rc_mode(
            params,
            &caps.encode_caps,
            min_q_index.div_ceil(5),
            max_q_index / 5,
            &structure,
        );
        debug!(?rc_mode, "selected rate control mode");

        let level = if caps.av1_caps.max_level < MAX_LEVEL {
            caps.av1_caps.max_level
        } else {
            MAX_LEVEL
        };

        let use_128x128_superblock = !caps
            .av1_caps
            .superblock_sizes
            .contains(vk::VideoEncodeAV1SuperblockSizeFlagsKHR::TYPE_64);

        let mut color_config = vk::native::StdVideoAV1ColorConfig {
            BitDepth: 8,
            subsampling_x: 1,
            subsampling_y: 1,
            color_primaries:
                vk::native::StdVideoAV1ColorPrimaries_STD_VIDEO_AV1_COLOR_PRIMARIES_BT_709,
            transfer_characteristics:
                vk::native::StdVideoAV1TransferCharacteristics_STD_VIDEO_AV1_TRANSFER_CHARACTERISTICS_BT_709,
            matrix_coefficients:
                vk::native::StdVideoAV1MatrixCoefficients_STD_VIDEO_AV1_MATRIX_COEFFICIENTS_BT_709,
            chroma_sample_position:
                vk::native::StdVideoAV1ChromaSamplePosition_STD_VIDEO_AV1_CHROMA_SAMPLE_POSITION_UNKNOWN,
            ..unsafe { std::mem::zeroed() }
        };
        color_config.flags.set_color_description_present_flag(1);
        color_config.flags.set_color_range(0); // Narrow range.

        let mut seq_header = vk::native::StdVideoAV1SequenceHeader {
            seq_profile: vk::native::StdVideoAV1Profile_STD_VIDEO_AV1_PROFILE_MAIN,
            frame_width_bits_minus_1: bits_minus_1(params.width - 1),
            frame_height_bits_minus_1: bits_minus_1(params.height - 1),
            max_frame_width_minus_1: (params.width - 1) as u16,
            max_frame_height_minus_1: (params.height - 1) as u16,
            order_hint_bits_minus_1: 7,
            // 2 means "chosen per frame" for both of these.
            seq_force_integer_mv: 2,
            seq_force_screen_content_tools: 2,
            pColorConfig: &color_config,
            pTimingInfo: std::ptr::null(),
            ..unsafe { std::mem::zeroed() }
        };
        seq_header.flags.set_enable_order_hint(1);
        seq_header.flags.set_enable_cdef(1);
        seq_header
            .flags
            .set_use_128x128_superblock(use_128x128_superblock as u32);

        let operating_points = [vk::native::StdVideoEncodeAV1OperatingPointInfo {
            operating_point_idc: 0,
            seq_level_idx: level as u8,
            seq_tier: 0,
            ..unsafe { std::mem::zeroed() }
        }];

        let mut session_params = vk::VideoEncodeAV1SessionParametersCreateInfoKHR::default()
            .std_sequence_header(&seq_header)
            .std_operating_points(&operating_points);

        let inner = super::EncoderInner::new(
            vk.clone(),
            params.width,
            params.height,
            framerate,
            params.in_flight_frames,
            structure.required_dpb_size(),
            profile.as_mut(),
            caps.video_caps,
            &mut session_params as *mut _ as *mut std::ffi::c_void,
            sink,
        )?;

        // The driver writes the sequence header OBU for us.
        let seq_header_obu = unsafe {
            let get_info = vk::VideoEncodeSessionParametersGetInfoKHR::default()
                .video_session_parameters(inner.session_params);
            let mut feedback_info = vk::VideoEncodeSessionParametersFeedbackInfoKHR::default();

            let size = encode_device
                .get_encoded_video_session_parameters_len(&get_info, Some(&mut feedback_info))
                .context("vkGetEncodedVideoSessionParametersKHR (query size)")?;
            let mut out = vec![MaybeUninit::<u8>::uninit(); size];
            encode_device
                .get_encoded_video_session_parameters(&get_info, Some(&mut feedback_info), &mut out)
                .context("vkGetEncodedVideoSessionParametersKHR")?;

            // The size query returns exactly the number of bytes written.
            Bytes::copy_from_slice(std::slice::from_raw_parts(
                out.as_ptr().cast::<u8>(),
                out.len(),
            ))
        };

        if seq_header_obu.is_empty() {
            bail!("failed to generate the AV1 sequence header");
        }
        trace!("generated {} bytes of av1 sequence header", seq_header_obu.len());

        let mut keyframe_headers = BytesMut::from(&TEMPORAL_DELIMITER[..]);
        keyframe_headers.extend_from_slice(&seq_header_obu);

        warn!(level, "AV1 encoding is experimental");

        Ok(Self {
            inner,
            profile,
            rc_mode,
            min_q_index,
            max_q_index,
            structure,
            last: Av1Metadata::default(),
            render_size: ((params.width - 1) as u16, (params.height - 1) as u16),
            order_hint: 0,
            keyframe_headers: keyframe_headers.freeze(),
        })
    }

    fn q_index(&self, qp: u32) -> u32 {
        qp_to_q_index(qp).clamp(self.min_q_index, self.max_q_index)
    }

    pub unsafe fn submit_encode(
        &mut self,
        input: &VkImage,
        tp_acquire: VkTimelinePoint,
        tp_release: VkTimelinePoint,
    ) -> anyhow::Result<()> {
        let frame_state = self.structure.next_frame();
        if frame_state.is_keyframe {
            self.order_hint = 0;
        }

        let mut av1_rc_layers = Vec::new();
        let mut rc_layers = Vec::new();

        if let RateControlMode::Vbr(vbr) = self.rc_mode {
            let settings = vbr.layer(0);
            let q_index = |qp| {
                let q = self.q_index(qp);
                vk::VideoEncodeAV1QIndexKHR {
                    intra_q_index: q,
                    predictive_q_index: q,
                    bipredictive_q_index: q,
                }
            };

            av1_rc_layers.push(
                vk::VideoEncodeAV1RateControlLayerInfoKHR::default()
                    .use_min_q_index(true)
                    .min_q_index(q_index(settings.min_qp))
                    .use_max_q_index(true)
                    .max_q_index(q_index(settings.max_qp)),
            );

            let mut layer_info = vk::VideoEncodeRateControlLayerInfoKHR::default()
                .max_bitrate(settings.peak_bitrate)
                .average_bitrate(settings.average_bitrate)
                .frame_rate_numerator(self.inner.framerate)
                .frame_rate_denominator(1);
            layer_info.p_next = &mut av1_rc_layers[0] as *mut _ as *mut std::ffi::c_void;
            rc_layers.push(layer_info);
        }

        let mut av1_rc_info = vk::VideoEncodeAV1RateControlInfoKHR::default()
            .flags(
                vk::VideoEncodeAV1RateControlFlagsKHR::REGULAR_GOP
                    | vk::VideoEncodeAV1RateControlFlagsKHR::REFERENCE_PATTERN_FLAT,
            )
            .gop_frame_count(self.structure.gop_size)
            .key_frame_period(self.structure.gop_size)
            .consecutive_bipredictive_frame_count(0)
            .temporal_layer_count(rc_layers.len() as u32);

        let vbv_size = match self.rc_mode {
            RateControlMode::Vbr(settings) => settings.vbv_size_ms,
            _ => 0,
        };

        let mut rc_info = vk::VideoEncodeRateControlInfoKHR::default()
            .rate_control_mode(self.rc_mode.as_vk_flags())
            .virtual_buffer_size_in_ms(vbv_size);
        if !rc_layers.is_empty() {
            rc_info = rc_info.layers(&rc_layers);
        }
        rc_info.p_next = <*mut _>::cast(&mut av1_rc_info);

        let frame_type = if frame_state.is_keyframe {
            vk::native::StdVideoAV1FrameType_STD_VIDEO_AV1_FRAME_TYPE_KEY
        } else {
            vk::native::StdVideoAV1FrameType_STD_VIDEO_AV1_FRAME_TYPE_INTER
        };

        // Filters and tools the driver may read; zeroed means "off/default".
        let quantization: vk::native::StdVideoAV1Quantization = unsafe { std::mem::zeroed() };
        let loop_filter: vk::native::StdVideoAV1LoopFilter = unsafe { std::mem::zeroed() };
        let cdef: vk::native::StdVideoAV1CDEF = unsafe { std::mem::zeroed() };
        let loop_restoration: vk::native::StdVideoAV1LoopRestoration =
            unsafe { std::mem::zeroed() };
        let global_motion: vk::native::StdVideoAV1GlobalMotion = unsafe { std::mem::zeroed() };

        // Every frame refreshes all eight reference slots, so on inter frames
        // they all hold the previous picture, and every reference name points
        // at slot 0.
        let ref_order_hint = if frame_state.is_keyframe {
            [0; 8]
        } else {
            [self.last.order_hint; 8]
        };

        let mut std_pic_info = vk::native::StdVideoEncodeAV1PictureInfo {
            frame_type,
            order_hint: self.order_hint,
            primary_ref_frame: PRIMARY_REF_NONE,
            refresh_frame_flags: 0xff,
            render_width_minus_1: self.render_size.0,
            render_height_minus_1: self.render_size.1,
            interpolation_filter:
                vk::native::StdVideoAV1InterpolationFilter_STD_VIDEO_AV1_INTERPOLATION_FILTER_EIGHTTAP,
            TxMode: vk::native::StdVideoAV1TxMode_STD_VIDEO_AV1_TX_MODE_SELECT,
            ref_order_hint,
            ref_frame_idx: [0; 7],
            pTileInfo: std::ptr::null(),
            pQuantization: &quantization,
            pSegmentation: std::ptr::null(),
            pLoopFilter: &loop_filter,
            pCDEF: &cdef,
            pLoopRestoration: &loop_restoration,
            pGlobalMotion: &global_motion,
            pExtensionHeader: std::ptr::null(),
            pBufferRemovalTimes: std::ptr::null(),
            ..unsafe { std::mem::zeroed() }
        };
        std_pic_info.flags.set_show_frame(1);
        std_pic_info.flags.set_render_and_frame_size_different(1);

        let mut reference_name_slot_indices = [-1; vk::MAX_VIDEO_AV1_REFERENCES_PER_FRAME_KHR];
        for id in &frame_state.ref_ids {
            let pic = self
                .inner
                .dpb
                .get_pic(*id)
                .ok_or(anyhow::anyhow!("ref pic {id} missing from dpb"))?;
            // Index 0 is LAST_FRAME.
            reference_name_slot_indices[0] = pic.index as i32;
        }

        let (prediction_mode, rate_control_group) = if frame_state.is_keyframe {
            (
                vk::VideoEncodeAV1PredictionModeKHR::INTRA_ONLY,
                vk::VideoEncodeAV1RateControlGroupKHR::INTRA,
            )
        } else {
            (
                vk::VideoEncodeAV1PredictionModeKHR::SINGLE_REFERENCE,
                vk::VideoEncodeAV1RateControlGroupKHR::PREDICTIVE,
            )
        };

        let constant_q_index = match &self.rc_mode {
            RateControlMode::ConstantQp(qp) => self.q_index(qp.layer(0)),
            _ => 0,
        };

        let mut av1_pic_info = vk::VideoEncodeAV1PictureInfoKHR::default()
            .prediction_mode(prediction_mode)
            .rate_control_group(rate_control_group)
            .constant_q_index(constant_q_index)
            .std_picture_info(&std_pic_info)
            .reference_name_slot_indices(reference_name_slot_indices);

        let mut std_ref_infos = frame_state
            .ref_ids
            .iter()
            .map(|_| vk::native::StdVideoEncodeAV1ReferenceInfo {
                frame_type: self.last.frame_type,
                OrderHint: self.last.order_hint,
                ..unsafe { std::mem::zeroed() }
            })
            .collect::<Vec<_>>();

        let mut ref_info = std_ref_infos
            .iter_mut()
            .map(|info| vk::VideoEncodeAV1DpbSlotInfoKHR::default().std_reference_info(info))
            .collect::<Vec<_>>();

        let setup_std_ref_info = vk::native::StdVideoEncodeAV1ReferenceInfo {
            frame_type,
            OrderHint: self.order_hint,
            ..unsafe { std::mem::zeroed() }
        };
        let mut setup_info =
            vk::VideoEncodeAV1DpbSlotInfoKHR::default().std_reference_info(&setup_std_ref_info);

        let insert = if frame_state.is_keyframe {
            self.keyframe_headers.clone()
        } else {
            Bytes::from_static(&TEMPORAL_DELIMITER)
        };

        let mut ref_pnext = ref_info
            .iter_mut()
            .map(|r| r as *mut _ as *mut std::ffi::c_void)
            .collect::<Vec<*mut std::ffi::c_void>>();

        unsafe {
            self.inner.submit_encode(
                input,
                tp_acquire,
                tp_release,
                &frame_state,
                &mut rc_info as *mut _ as *mut std::ffi::c_void,
                &mut av1_pic_info as *mut _ as *mut std::ffi::c_void,
                &mut setup_info as *mut _ as *mut std::ffi::c_void,
                &mut ref_pnext,
                Some(insert),
            )?;
        }

        self.last = Av1Metadata {
            frame_type,
            order_hint: self.order_hint,
        };
        self.order_hint = self.order_hint.wrapping_add(1);

        Ok(())
    }

    pub fn input_format(&self) -> vk::Format {
        self.inner.input_format
    }

    pub fn input_storage(&self) -> bool {
        self.inner.input_storage
    }

    pub fn describe(&self) -> super::EncoderDescription {
        super::EncoderDescription {
            rate_control: self.rc_mode.to_string(),
            temporal_layers: self.structure.layers,
            input_storage: self.inner.input_storage,
        }
    }

    pub fn stats(&self) -> &super::stats::EncodeStats {
        &self.inner.stats
    }

    pub fn create_input_image(&mut self) -> anyhow::Result<VkImage> {
        self.inner.create_input_image(self.profile.as_mut())
    }

    pub fn request_refresh(&mut self) {
        self.structure.request_refresh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_dimension_bits() {
        // 1920 - 1 = 1919 needs 11 bits.
        assert_eq!(bits_minus_1(1919), 10);
        assert_eq!(bits_minus_1(1079), 10);
        assert_eq!(bits_minus_1(1023), 9);
        assert_eq!(bits_minus_1(1024), 10);
    }

    #[test]
    fn q_index_scale() {
        assert_eq!(qp_to_q_index(0), 0);
        assert_eq!(qp_to_q_index(22), 110);
        assert_eq!(qp_to_q_index(51), 255);
        assert_eq!(qp_to_q_index(60), 255);
    }
}
