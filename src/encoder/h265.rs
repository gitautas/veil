use std::mem::MaybeUninit;
use std::sync::Arc;

use anyhow::{Context, bail};
use ash::vk;
use bytes::Bytes;
use tracing::{debug, trace};

use super::gop_structure::HierarchicalP;
use super::rate_control::{self, RateControlMode};
use crate::codec::VideoCodec;
use crate::color::VideoProfile;
use crate::{session::control::VideoStreamParams, vulkan::*};

vk_chain! {
    pub struct H265EncodeProfile<'a> {
        pub profile_info: vk::VideoProfileInfoKHR<'a>,
        pub encode_usage_info: vk::VideoEncodeUsageInfoKHR<'a>,
        pub h265_profile: vk::VideoEncodeH265ProfileInfoKHR<'a>,
    }
}

vk_chain! {
    pub struct H265EncodeCapabilities<'a> {
        pub video_caps: vk::VideoCapabilitiesKHR<'a>,
        pub encode_caps: vk::VideoEncodeCapabilitiesKHR<'a>,
        pub h265_caps: vk::VideoEncodeH265CapabilitiesKHR<'a>,
    }
}

vk_chain! {
    pub struct H265QualityLevelProperties<'a> {
        pub props: vk::VideoEncodeQualityLevelPropertiesKHR<'a>,
        pub h265_props: vk::VideoEncodeH265QualityLevelPropertiesKHR<'a>,
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct H265Metadata {
    pic_type: u32,
    pic_order_cnt: i32,
    ref_count: u32,
}

pub struct H265Encoder {
    inner: super::EncoderInner,
    profile: H265EncodeProfile,
    rc_mode: RateControlMode,
    /// The preset's rate control, before any bitrate limit.
    base_rc: RateControlMode,
    /// A changed rate control to apply on the next frame.
    pending_rc: Option<RateControlMode>,

    structure: HierarchicalP,
    pic_metadata: Vec<H265Metadata>, // Indexed by layer.
    idr_num: u32,
    frame_num: u32,

    headers: Bytes,
}

impl H265Encoder {
    pub fn new(
        vk: Arc<VkContext>,
        params: VideoStreamParams,
        framerate: u32,
        sink: impl super::Sink,
    ) -> anyhow::Result<Self> {
        let video_apis = vk.video_apis.as_ref().unwrap();
        let video_instance = &video_apis.video_instance;
        let _video_device = &video_apis.video_device;
        let encode_device = &video_apis.encode_device;

        let op = vk::VideoCodecOperationFlagsKHR::ENCODE_H265;
        let (profile, profile_idc) = match params.profile {
            VideoProfile::Hd => (super::default_profile(op), 1), // Main
            VideoProfile::Hdr10 => (super::default_hdr10_profile(op), 2), // Main10
        };

        let h265_profile_info =
            vk::VideoEncodeH265ProfileInfoKHR::default().std_profile_idc(profile_idc);

        let mut profile = H265EncodeProfile::new(
            profile,
            super::default_encode_usage(vk.device_info.driver_version.clone()),
            h265_profile_info,
        );

        let mut caps = H265EncodeCapabilities::default();

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
        trace!("h265 capabilities: {:#?}", caps.h265_caps);

        let _quality_level = caps.encode_caps.max_quality_levels - 1;
        let quality_props = H265QualityLevelProperties::default();

        // NOTE: In newer ash, quality-level property helpers may move or change signatures.
        // This is non-critical for basic bring-up, so skip the query for now and rely on caps.
        //
        // If you want this back, we can reintroduce it once we confirm the exact ash API surface.
        // unsafe {
        //     let get_info = vk::PhysicalDeviceVideoEncodeQualityLevelInfoKHR::default()
        //         .video_profile(&profile.profile_info)
        //         .quality_level(quality_level);
        //
        //     encode_device.get_physical_device_video_encode_quality_level_properties(
        //         vk.device_info.pdevice,
        //         &get_info,
        //         quality_props.as_mut(),
        //     )?;
        // }

        trace!("quality level properties: {:#?}", quality_props.props);
        trace!(
            "h265 quality level properties: {:#?}",
            quality_props.h265_props
        );

        let structure = super::default_structure(
            VideoCodec::H265,
            caps.h265_caps
                .max_sub_layer_count
                .min(caps.encode_caps.max_rate_control_layers),
            caps.video_caps.max_dpb_slots,
            params.gop_size,
        )?;

        let rc_mode = rate_control::select_rc_mode(
            params,
            &caps.encode_caps,
            caps.h265_caps.min_qp.try_into().unwrap_or(17),
            caps.h265_caps.max_qp.try_into().unwrap_or(50),
            &structure,
        );

        debug!(?rc_mode, "selected rate control mode");

        // TODO check more caps
        // TODO autoselect level
        let level_idc = vk::native::StdVideoH265LevelIdc_STD_VIDEO_H265_LEVEL_IDC_5_2;
        if caps.h265_caps.max_level_idc != 0 && caps.h265_caps.max_level_idc < level_idc {
            bail!("video resolution too large for hardware");
        }

        const CTB_SIZES: [(vk::VideoEncodeH265CtbSizeFlagsKHR, usize); 3] = [
            (vk::VideoEncodeH265CtbSizeFlagsKHR::TYPE_16, 16),
            (vk::VideoEncodeH265CtbSizeFlagsKHR::TYPE_32, 32),
            (vk::VideoEncodeH265CtbSizeFlagsKHR::TYPE_64, 64),
        ];

        let min_ctb = CTB_SIZES
            .iter()
            .filter(|(flag, _)| caps.h265_caps.ctb_sizes.contains(*flag))
            .map(|(_, size)| *size)
            .min()
            .expect("no ctb size found");

        let max_ctb = CTB_SIZES
            .iter()
            .filter(|(flag, _)| caps.h265_caps.ctb_sizes.contains(*flag))
            .map(|(_, size)| *size)
            .max()
            .expect("no ctb size found");

        const TBS_SIZES: [(vk::VideoEncodeH265TransformBlockSizeFlagsKHR, usize); 4] = [
            (vk::VideoEncodeH265TransformBlockSizeFlagsKHR::TYPE_4, 4),
            (vk::VideoEncodeH265TransformBlockSizeFlagsKHR::TYPE_8, 8),
            (vk::VideoEncodeH265TransformBlockSizeFlagsKHR::TYPE_16, 16),
            (vk::VideoEncodeH265TransformBlockSizeFlagsKHR::TYPE_32, 32),
        ];

        let min_tbs = TBS_SIZES
            .iter()
            .filter(|(flag, _)| caps.h265_caps.transform_block_sizes.contains(*flag))
            .map(|(_, size)| *size)
            .min()
            .expect("no tbs size found");

        let max_tbs = TBS_SIZES
            .iter()
            .filter(|(flag, _)| caps.h265_caps.transform_block_sizes.contains(*flag))
            .map(|(_, size)| *size)
            .max()
            .expect("no tbs size found");

        let aligned_width = params
            .width
            .next_multiple_of(caps.encode_caps.encode_input_picture_granularity.width);
        let aligned_height = params
            .height
            .next_multiple_of(caps.encode_caps.encode_input_picture_granularity.height);

        trace!(
            min_ctb,
            max_ctb, min_tbs, max_tbs, aligned_width, aligned_height, "block sizes",
        );

        let crop_right = (aligned_width - params.width) / 2;
        let crop_bottom = (aligned_height - params.height) / 2;

        trace!("crop right: {}, bottom: {}", crop_right, crop_bottom);

        let (colour_primaries, transfer_characteristics, matrix_coeffs) = match params.profile {
            VideoProfile::Hd => (1, 1, 1),
            VideoProfile::Hdr10 => (9, 16, 9),
        };

        let mut vui = vk::native::StdVideoH265SequenceParameterSetVui {
            colour_primaries,
            transfer_characteristics,
            matrix_coeffs,
            // Unspecified.
            video_format: 5,
            ..unsafe { std::mem::zeroed() }
        };

        vui.flags.set_video_signal_type_present_flag(1);
        vui.flags.set_colour_description_present_flag(1);
        vui.flags.set_video_full_range_flag(0); // Narrow range.

        let ptl = vk::native::StdVideoH265ProfileTierLevel {
            general_profile_idc: profile_idc,
            general_level_idc: level_idc,
            ..unsafe { std::mem::zeroed() }
        };

        // ptl.flags.set_general_progressive_source_flag(1);
        // ptl.flags.set_general_interlaced_source_flag(0);

        let layers_minus_1 = (structure.layers - 1) as u8;
        let mut pbm: vk::native::StdVideoH265DecPicBufMgr = unsafe { std::mem::zeroed() };
        // With sub_layer_ordering_info_present_flag set, every sub-layer's
        // values are signalled, so fill them all: a zero here claims a
        // one-picture DPB for that sub-layer, which strict decoders
        // (VideoToolbox) reject frames over. Using the full DPB size for each
        // is valid, since only the base layer is referenced.
        for i in 0..=layers_minus_1 as usize {
            pbm.max_dec_pic_buffering_minus1[i] = (structure.required_dpb_size() - 1) as u8;
            // No picture reordering.
            pbm.max_num_reorder_pics[i] = 0;
            pbm.max_latency_increase_plus1[i] = 0;
        }

        let mut vps = vk::native::StdVideoH265VideoParameterSet {
            vps_max_sub_layers_minus1: layers_minus_1,
            pDecPicBufMgr: &pbm,
            pHrdParameters: std::ptr::null(),
            pProfileTierLevel: &ptl,
            ..unsafe { std::mem::zeroed() }
        };

        vps.flags.set_vps_sub_layer_ordering_info_present_flag(1);
        vps.flags.set_vps_temporal_id_nesting_flag(1);

        let min_cb = 8_u8;
        let max_cb = max_ctb;

        let max_transform_hierarchy_depth = (max_ctb.ilog2() - min_tbs.ilog2()) as u8;

        let bit_depth = match params.profile {
            VideoProfile::Hd => 8,
            VideoProfile::Hdr10 => 10,
        };

        let mut sps = vk::native::StdVideoH265SequenceParameterSet {
            chroma_format_idc:
                vk::native::StdVideoH265ChromaFormatIdc_STD_VIDEO_H265_CHROMA_FORMAT_IDC_420,
            pic_width_in_luma_samples: aligned_width,
            pic_height_in_luma_samples: aligned_height,
            sps_max_sub_layers_minus1: layers_minus_1,
            bit_depth_luma_minus8: bit_depth - 8,
            bit_depth_chroma_minus8: bit_depth - 8,
            log2_max_pic_order_cnt_lsb_minus4: 4,
            log2_min_luma_coding_block_size_minus3: (min_cb.ilog2() - 3) as u8,
            log2_diff_max_min_luma_coding_block_size: (max_cb.ilog2() - min_cb.ilog2()) as u8,
            log2_min_luma_transform_block_size_minus2: (min_tbs.ilog2() - 2) as u8,
            log2_diff_max_min_luma_transform_block_size: (max_tbs.ilog2() - min_tbs.ilog2()) as u8,
            max_transform_hierarchy_depth_inter: max_transform_hierarchy_depth,
            max_transform_hierarchy_depth_intra: max_transform_hierarchy_depth,
            conf_win_right_offset: crop_right,
            conf_win_bottom_offset: crop_bottom,
            pProfileTierLevel: &ptl,
            pDecPicBufMgr: &pbm,
            pSequenceParameterSetVui: &vui,
            ..unsafe { std::mem::zeroed() }
        };

        sps.flags.set_conformance_window_flag(1);
        sps.flags.set_vui_parameters_present_flag(1);
        sps.flags.set_sps_temporal_id_nesting_flag(1);
        sps.flags.set_sps_sub_layer_ordering_info_present_flag(1);

        if caps
            .h265_caps
            .std_syntax_flags
            .contains(vk::VideoEncodeH265StdFlagsKHR::SAMPLE_ADAPTIVE_OFFSET_ENABLED_FLAG_SET)
        {
            sps.flags.set_sample_adaptive_offset_enabled_flag(1);
        }

        if caps
            .h265_caps
            .std_syntax_flags
            .contains(vk::VideoEncodeH265StdFlagsKHR::TRANSFORM_SKIP_ENABLED_FLAG_SET)
        {
            // The Slang/Vulkan headers we compile against may not expose a dedicated
            // transform-skip flag setter on the generated StdVideoH265SpsFlags type.
            // Keep this capability check for future use, but don't attempt to set an
            // unavailable flag method.
        }

        let pps = vk::native::StdVideoH265PictureParameterSet {
            ..unsafe { std::mem::zeroed() }
        };

        let sps = [sps];
        let pps = [pps];
        let vps = [vps];

        let h265_add_info = vk::VideoEncodeH265SessionParametersAddInfoKHR::default()
            .std_vp_ss(&vps)
            .std_sp_ss(&sps)
            .std_pp_ss(&pps);
        let mut session_params = vk::VideoEncodeH265SessionParametersCreateInfoKHR::default()
            .parameters_add_info(&h265_add_info)
            .max_std_vps_count(1)
            .max_std_pps_count(1)
            .max_std_sps_count(1);

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

        // Generate encoded stream headers.
        //
        // Newer ash:
        // - does not provide `.push_next(...)` on these structs
        // - `get_encoded_video_session_parameters` writes into a caller-provided byte buffer
        //   and returns `()`
        let headers = unsafe {
            let mut h265_get_info = vk::VideoEncodeH265SessionParametersGetInfoKHR::default()
                .write_std_vps(true)
                .write_std_sps(true)
                .write_std_pps(true);

            let mut h265_feedback_info =
                vk::VideoEncodeH265SessionParametersFeedbackInfoKHR::default();

            let mut feedback_info = vk::VideoEncodeSessionParametersFeedbackInfoKHR::default();
            feedback_info.p_next = &mut h265_feedback_info as *mut _ as *mut std::ffi::c_void;

            let mut get_info = vk::VideoEncodeSessionParametersGetInfoKHR::default()
                .video_session_parameters(inner.session_params);
            get_info.p_next = &mut h265_get_info as *mut _ as *mut std::ffi::c_void;

            // Query the exact size needed first
            let size = encode_device
                .get_encoded_video_session_parameters_len(&get_info, Some(&mut feedback_info))
                .context("vkGetEncodedVideoSessionParametersKHR (query size)")?;

            // Allocate the exact buffer size the driver needs
            let mut out = vec![MaybeUninit::<u8>::uninit(); size];

            encode_device
                .get_encoded_video_session_parameters(&get_info, Some(&mut feedback_info), &mut out)
                .context("vkGetEncodedVideoSessionParametersKHR")?;

            // The size query returns exactly the number of bytes the driver
            // writes, so the whole buffer is initialized on success.
            let out_init = std::slice::from_raw_parts(out.as_ptr().cast::<u8>(), out.len());
            Bytes::copy_from_slice(out_init)
        };

        if headers.is_empty() {
            bail!("failed to generate sps/pps/vps");
        } else {
            trace!("generated {} bytes of h265 headers", headers.len());
        }

        let pic_metadata = vec![H265Metadata::default(); structure.layers as usize];

        Ok(Self {
            inner,
            profile,
            base_rc: rc_mode.clone(),
            pending_rc: None,
            rc_mode,
            structure,
            pic_metadata,
            idr_num: 0,
            frame_num: 0,
            headers: Bytes::copy_from_slice(&headers),
        })
    }

    pub unsafe fn submit_encode(
        &mut self,
        input: &VkImage,
        tp_acquire: VkTimelinePoint,
        tp_release: VkTimelinePoint,
    ) -> anyhow::Result<()> {
        // Rust 2024 note:
        // Even inside an `unsafe fn`, the body is treated as safe by default.
        // Wrap individual unsafe operations in `unsafe { ... }` blocks.
        let frame_state = self.structure.next_frame();
        if frame_state.is_keyframe {
            self.idr_num += 1;
            self.frame_num = 0;
        }

        // The rate control in effect, and a new one to switch to (e.g. a
        // network bitrate limit changed).
        let mut rc = self.rc_chain(&self.rc_mode);
        let mut rc_update = self.pending_rc.take().map(|mode| {
            let chain = self.rc_chain(&mode);
            self.rc_mode = mode;
            chain
        });

        let weight_table: vk::native::StdVideoEncodeH265WeightTable = unsafe { std::mem::zeroed() };

        let slice_type = if frame_state.is_keyframe {
            vk::native::StdVideoH265SliceType_STD_VIDEO_H265_SLICE_TYPE_I
        } else {
            vk::native::StdVideoH265SliceType_STD_VIDEO_H265_SLICE_TYPE_P
        };

        let pic_type = if frame_state.is_keyframe {
            vk::native::StdVideoH265PictureType_STD_VIDEO_H265_PICTURE_TYPE_IDR
        } else {
            vk::native::StdVideoH265PictureType_STD_VIDEO_H265_PICTURE_TYPE_P
        };

        let std_slice_header = vk::native::StdVideoEncodeH265SliceSegmentHeader {
            slice_type,
            pWeightTable: &weight_table,
            MaxNumMergeCand: 5, // Decoders complain if this is zero. The max value is 5.
            ..unsafe { std::mem::zeroed() }
        };

        let slice_segment_info = [vk::VideoEncodeH265NaluSliceSegmentInfoKHR::default()
            .std_slice_segment_header(&std_slice_header)
            .constant_qp(if let RateControlMode::ConstantQp(qp) = self.rc_mode {
                qp.layer(frame_state.id) as i32
            } else {
                0
            })];

        let mut ref_lists_info = vk::native::StdVideoEncodeH265ReferenceListsInfo {
            RefPicList0: [u8::MAX; 15],
            RefPicList1: [u8::MAX; 15],
            ..unsafe { std::mem::zeroed() }
        };

        for (idx, id) in frame_state.ref_ids.iter().enumerate() {
            let pic = self
                .inner
                .dpb
                .get_pic(*id)
                .ok_or(anyhow::anyhow!("ref pic {id} missing from dpb"))?;

            ref_lists_info.RefPicList0[idx] = pic.index as u8;
        }

        // For each frame, we have to tell the decoder which pictures will be
        // used as references in the future, in addition to those that are
        // references for this frame.
        let mut ref_ids = if frame_state.is_keyframe {
            // If we're outputting a keyframe, clear the forward reference counts.
            for md in &mut self.pic_metadata {
                md.ref_count = 0;
            }

            Vec::new()
        } else {
            self.pic_metadata
                .iter_mut()
                .enumerate()
                .filter_map(|(id, md)| {
                    let id = id as u32;
                    if md.ref_count == 0 {
                        None
                    } else if frame_state.ref_ids.contains(&id) {
                        md.ref_count -= 1;
                        Some((id, true))
                    } else {
                        Some((id, false))
                    }
                })
                .collect::<Vec<_>>()
        };

        // Sort in descending order of POC.
        ref_ids.sort_by_key(|(id, _)| {
            std::cmp::Reverse(self.pic_metadata[*id as usize].pic_order_cnt)
        });

        let mut short_term_refs = vk::native::StdVideoH265ShortTermRefPicSet {
            used_by_curr_pic_s0_flag: 0,
            num_negative_pics: ref_ids.len() as u8,
            // No forward refs.
            used_by_curr_pic_s1_flag: 0,
            num_positive_pics: 0,
            ..unsafe { std::mem::zeroed() }
        };

        let pic_order_cnt = frame_state.gop_position as i32;
        let mut delta_poc = 0;
        for (idx, (id, is_direct_ref)) in ref_ids.into_iter().enumerate() {
            // delta_poc accumulates for each step backwards in time we take.
            // So if a frame references the immediately preceding one and then a
            // frame four frames ago, the delta_poc values are 1 and 3.
            //
            // Taking the modulo allows us to reference frames across a GOP
            // boundary.
            delta_poc = (pic_order_cnt - self.pic_metadata[id as usize].pic_order_cnt)
                .rem_euclid(self.structure.gop_size as i32)
                - delta_poc;

            short_term_refs.delta_poc_s0_minus1[idx] = (delta_poc - 1) as u16;
            if is_direct_ref {
                short_term_refs.used_by_curr_pic_s0_flag |= 1 << idx;
            }
        }

        let mut std_pic_info = vk::native::StdVideoEncodeH265PictureInfo {
            pic_type,
            sps_video_parameter_set_id: 0,
            pps_seq_parameter_set_id: 0,
            pps_pic_parameter_set_id: 0,
            PicOrderCntVal: frame_state.gop_position as i32,
            TemporalId: frame_state.id as u8,
            pRefLists: &ref_lists_info,
            pShortTermRefPicSet: &short_term_refs,
            ..unsafe { std::mem::zeroed() }
        };

        std_pic_info
            .flags
            .set_IrapPicFlag(frame_state.is_keyframe as u32);
        std_pic_info
            .flags
            .set_is_reference((frame_state.forward_ref_count > 0) as u32);

        if frame_state.is_keyframe {
            std_pic_info.flags.set_pic_output_flag(1);
            std_pic_info.flags.set_no_output_of_prior_pics_flag(1);
        }

        let mut h265_pic_info = vk::VideoEncodeH265PictureInfoKHR::default()
            .std_picture_info(&std_pic_info)
            .nalu_slice_segment_entries(&slice_segment_info);

        let mut std_ref_infos = frame_state
            .ref_ids
            .iter()
            .map(|id| vk::native::StdVideoEncodeH265ReferenceInfo {
                pic_type: self.pic_metadata[*id as usize].pic_type,
                PicOrderCntVal: self.pic_metadata[*id as usize].pic_order_cnt,
                TemporalId: *id as u8,
                ..unsafe { std::mem::zeroed() }
            })
            .collect::<Vec<_>>();

        let mut ref_info = std_ref_infos
            .iter_mut()
            .map(|info| vk::VideoEncodeH265DpbSlotInfoKHR::default().std_reference_info(info))
            .collect::<Vec<_>>();

        let setup_std_ref_info = vk::native::StdVideoEncodeH265ReferenceInfo {
            pic_type,
            PicOrderCntVal: pic_order_cnt,
            TemporalId: frame_state.id as u8,
            ..unsafe { std::mem::zeroed() }
        };

        let mut setup_info =
            vk::VideoEncodeH265DpbSlotInfoKHR::default().std_reference_info(&setup_std_ref_info);

        let insert = if frame_state.is_keyframe {
            Some(self.headers.clone())
        } else {
            None
        };

        // Build pNext pointers for the codec-specific structs expected by the generic encoder path.
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
                rc.as_pnext(),
                rc_update.as_mut().map(|c| c.as_pnext()),
                &mut h265_pic_info as *mut _ as *mut std::ffi::c_void,
                &mut setup_info as *mut _ as *mut std::ffi::c_void,
                &mut ref_pnext,
                insert,
            )?;
        }

        // Save the reference info for the DPB slot we just wrote.
        self.pic_metadata[frame_state.id as usize] = H265Metadata {
            pic_type,
            pic_order_cnt,
            ref_count: frame_state.forward_ref_count,
        };

        // This is supposed to increment only for reference frames.
        if frame_state.forward_ref_count > 0 {
            self.frame_num += 1;
        }

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

    /// Rate control for `mode`, as Vulkan wants it.
    fn rc_chain(
        &self,
        mode: &RateControlMode,
    ) -> Box<
        super::RcChain<vk::VideoEncodeH265RateControlInfoKHR<'static>, vk::VideoEncodeH265RateControlLayerInfoKHR<'static>>,
    > {
        let pattern = if self.structure.layers > 1 {
            vk::VideoEncodeH265RateControlFlagsKHR::TEMPORAL_SUB_LAYER_PATTERN_DYADIC
        } else {
            vk::VideoEncodeH265RateControlFlagsKHR::REFERENCE_PATTERN_FLAT
        };

        let mut h265_layers = Vec::new();
        let mut layers = Vec::new();
        if let RateControlMode::Vbr(vbr) = mode {
            for layer in 0..self.structure.layers {
                let settings = vbr.layer(layer);
                let qp = |qp: u32| vk::VideoEncodeH265QpKHR { qp_i: qp as i32, qp_p: qp as i32, qp_b: qp as i32 };
                h265_layers.push(
                    vk::VideoEncodeH265RateControlLayerInfoKHR::default()
                        .use_min_qp(true)
                        .use_max_qp(true)
                        .min_qp(qp(settings.min_qp))
                        .max_qp(qp(settings.max_qp)),
                );
                let (fps_numerator, fps_denominator) =
                    self.structure.layer_framerate(layer, self.inner.framerate);
                layers.push(
                    vk::VideoEncodeRateControlLayerInfoKHR::default()
                        .max_bitrate(settings.peak_bitrate)
                        .average_bitrate(settings.average_bitrate)
                        .frame_rate_numerator(fps_numerator)
                        .frame_rate_denominator(fps_denominator),
                );
            }
        }

        let h265_info = vk::VideoEncodeH265RateControlInfoKHR::default()
            .gop_frame_count(self.structure.gop_size)
            .idr_period(self.structure.gop_size)
            .consecutive_b_frame_count(0)
            .sub_layer_count(layers.len() as u32)
            .flags(vk::VideoEncodeH265RateControlFlagsKHR::REGULAR_GOP | pattern);

        super::RcChain::new(mode, h265_layers, layers, h265_info)
    }

    pub fn set_bitrate_limit(&mut self, limit: Option<u64>) {
        let mode = self.base_rc.limited(limit);
        self.pending_rc = (mode != self.rc_mode).then_some(mode);
    }

    pub fn bitrates(&self) -> (Option<u64>, Option<u64>) {
        let current = self.pending_rc.as_ref().unwrap_or(&self.rc_mode);
        (current.average_bitrate(), self.base_rc.average_bitrate())
    }

    pub fn request_refresh(&mut self) {
        self.structure.request_refresh()
    }
}
