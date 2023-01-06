use sdp::{CodecSpec, PayloadParams as SdpPayloadParams};

use rtp::Pt;
use sdp::{Codec, FormatParams};

use super::MediaKind;

/// Group of parameters for a payload type (PT).
///
/// In the SDP a payload type has a number of associated parameters. See example below:
///
/// ```text
/// a=rtpmap:96 H264/90000
/// a=rtcp-fb:96 goog-remb
/// a=rtcp-fb:96 transport-cc
/// a=rtcp-fb:96 ccm fir
/// a=rtcp-fb:96 nack
/// a=rtcp-fb:96 nack pli
/// a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42001f
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadParams(SdpPayloadParams);

impl PayloadParams {
    pub(crate) fn new(p: SdpPayloadParams) -> Self {
        PayloadParams(p)
    }

    /// The payload type that groups these parameters.
    pub fn pt(&self) -> Pt {
        self.0.codec.pt
    }

    /// Whether these parameters are repairing some other set of parameters.
    /// This is used to, via PT, separate RTX resend streams from the main stream.
    pub fn pt_rtx(&self) -> Option<Pt> {
        self.0.resend
    }

    /// The codec for this group of parameters.
    pub fn codec(&self) -> Codec {
        self.0.codec.codec
    }

    /// Clock rate of the codec.
    pub fn clock_rate(&self) -> u32 {
        self.0.codec.clock_rate
    }

    /// Number of audio channels (if any).
    pub fn channels(&self) -> Option<u8> {
        self.0.codec.channels
    }

    /// Codec specific format parameters. This might carry additional config for
    /// things like h264.
    pub fn fmtp(&self) -> &FormatParams {
        &self.0.fmtps
    }

    pub(crate) fn inner(&self) -> &SdpPayloadParams {
        &self.0
    }

    pub(crate) fn match_score(&self, o: Self) -> Option<usize> {
        // we don't want to compare PT
        let pt = 0.into();
        let c0 = CodecSpec { pt, ..self.0.codec };
        let c1 = CodecSpec { pt, ..o.0.codec };

        if c0 == c1 && self.0.fmtps == o.0.fmtps {
            return Some(100);
        } else {
            // TODO: fuzzy matching.
        }

        // No match
        None
    }
}

/// Session config for all codecs.
#[derive(Debug, Clone, Default)]
pub struct CodecConfig {
    configs: Vec<PayloadParams>,
}

impl CodecConfig {
    /// Creates a new empty config.
    pub fn new() -> Self {
        CodecConfig::default()
    }

    /// Creates a new config with all default configurations enabled.
    pub fn new_with_defaults() -> Self {
        let mut c = Self::new();
        c.add_default_opus();

        c.add_default_vp8();
        c.add_default_h264();
        // c.add_default_av1();
        c.add_default_vp9();

        c
    }

    /// Clear all configured configs.
    pub fn clear(&mut self) {
        self.configs.clear();
    }

    pub(crate) fn matches(&self, c: &PayloadParams) -> bool {
        self.configs.iter().any(|x| {
            x.codec() == c.codec() && x.clock_rate() == c.clock_rate() && x.fmtp() == c.fmtp()
        })
    }

    /// Manually configure a payload type.
    pub fn add_config(
        &mut self,
        pt: Pt,
        resend: Option<Pt>,
        codec: Codec,
        clock_rate: u32,
        channels: Option<u8>,
        fmtps: FormatParams,
    ) {
        let (fb_transport_cc, fb_fir, fb_nack, fb_pli, resend) = if codec.is_video() {
            (true, true, true, true, resend)
        } else {
            (true, false, false, false, None)
        };

        let p = SdpPayloadParams {
            codec: CodecSpec {
                pt,
                codec,
                clock_rate,
                channels,
            },
            fmtps,
            resend,
            fb_transport_cc,
            fb_fir,
            fb_nack,
            fb_pli,
        };

        self.configs.push(PayloadParams::new(p));
    }

    /// Convenience for adding a h264 payload type.
    pub fn add_h264(
        &mut self,
        pt: Pt,
        resend: Option<Pt>,
        packetization_mode: bool,
        profile_level_id: u32,
    ) {
        self.add_config(
            pt,
            resend,
            Codec::H264,
            90_000,
            None,
            FormatParams {
                level_asymmetry_allowed: Some(true),
                packetization_mode: if packetization_mode { Some(1) } else { Some(0) },
                profile_level_id: Some(profile_level_id),
                ..Default::default()
            },
        )
    }

    /// Add a default OPUS payload type.
    pub fn add_default_opus(&mut self) {
        self.add_config(
            111.into(),
            None,
            Codec::Opus,
            48_000,
            Some(2),
            FormatParams {
                min_p_time: Some(10),
                use_inband_fec: Some(true),
                ..Default::default()
            },
        )
    }

    /// Add a default VP8 payload type.
    pub fn add_default_vp8(&mut self) {
        self.add_config(
            96.into(),
            Some(97.into()),
            Codec::Vp8,
            90_000,
            None,
            FormatParams::default(),
        )
    }

    /// Add a default H264 payload type.
    pub fn add_default_h264(&mut self) {
        const PARAMS: &[(u8, u8, bool, u32)] = &[
            (127, 121, true, 0x42001f),
            (125, 107, false, 0x42001f),
            (108, 109, true, 0x42e01f),
            (124, 120, false, 0x42e01f),
            (123, 119, true, 0x4d001f),
            (35, 36, false, 0x4d001f),
            (114, 115, true, 0x64001f),
        ];

        for p in PARAMS {
            self.add_h264(p.0.into(), Some(p.1.into()), p.2, p.3)
        }
    }

    // TODO: AV1 depacketizer/packetizer.
    //
    // /// Add a default AV1 payload type.
    // pub fn add_default_av1(&mut self) {
    //     self.add_config(
    //         41.into(),
    //         Some(42.into()),
    //         Codec::Av1,
    //         90_000,
    //         None,
    //         FormatParams::default(),
    //     )
    // }

    /// Add a default VP9 payload type.
    pub fn add_default_vp9(&mut self) {
        self.add_config(
            98.into(),
            Some(99.into()),
            Codec::Vp9,
            90_000,
            None,
            FormatParams {
                profile_id: Some(0),
                ..Default::default()
            },
        );
        self.add_config(
            100.into(),
            Some(101.into()),
            Codec::Vp9,
            90_000,
            None,
            FormatParams {
                profile_id: Some(2),
                ..Default::default()
            },
        );
    }

    pub(crate) fn all_for_kind(&self, kind: MediaKind) -> impl Iterator<Item = &PayloadParams> {
        self.configs.iter().filter(move |c| {
            if kind == MediaKind::Video {
                c.codec().is_video()
            } else {
                c.codec().is_audio()
            }
        })
    }
}
