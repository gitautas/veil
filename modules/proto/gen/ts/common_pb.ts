// @generated by protoc-gen-es v1.10.0 with parameter "target=ts"
// @generated from file common.proto (package veil, syntax proto3)
/* eslint-disable */
// @ts-nocheck

import type { BinaryReadOptions, FieldList, JsonReadOptions, JsonValue, PartialMessage, PlainMessage } from "@bufbuild/protobuf";
import { Message, proto3 } from "@bufbuild/protobuf";

/**
 * @generated from message veil.Resolution
 */
export class Resolution extends Message<Resolution> {
  /**
   * @generated from field: int32 width = 1;
   */
  width = 0;

  /**
   * @generated from field: int32 height = 2;
   */
  height = 0;

  constructor(data?: PartialMessage<Resolution>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.Resolution";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "width", kind: "scalar", T: 5 /* ScalarType.INT32 */ },
    { no: 2, name: "height", kind: "scalar", T: 5 /* ScalarType.INT32 */ },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): Resolution {
    return new Resolution().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): Resolution {
    return new Resolution().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): Resolution {
    return new Resolution().fromJsonString(jsonString, options);
  }

  static equals(a: Resolution | PlainMessage<Resolution> | undefined, b: Resolution | PlainMessage<Resolution> | undefined): boolean {
    return proto3.util.equals(Resolution, a, b);
  }
}

/**
 * @generated from message veil.Display
 */
export class Display extends Message<Display> {
  /**
   * @generated from field: veil.Resolution resolution = 1;
   */
  resolution?: Resolution;

  /**
   * @generated from field: int32 refresh_rate = 2;
   */
  refreshRate = 0;

  constructor(data?: PartialMessage<Display>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.Display";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "resolution", kind: "message", T: Resolution },
    { no: 2, name: "refresh_rate", kind: "scalar", T: 5 /* ScalarType.INT32 */ },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): Display {
    return new Display().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): Display {
    return new Display().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): Display {
    return new Display().fromJsonString(jsonString, options);
  }

  static equals(a: Display | PlainMessage<Display> | undefined, b: Display | PlainMessage<Display> | undefined): boolean {
    return proto3.util.equals(Display, a, b);
  }
}

/**
 * @generated from message veil.VideoStream
 */
export class VideoStream extends Message<VideoStream> {
  /**
   * @generated from field: veil.Display display = 1;
   */
  display?: Display;

  /**
   * @generated from field: veil.VideoStream.Codec codec = 2;
   */
  codec = VideoStream_Codec.CODEC_UNSPECIFIED;

  constructor(data?: PartialMessage<VideoStream>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.VideoStream";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "display", kind: "message", T: Display },
    { no: 2, name: "codec", kind: "enum", T: proto3.getEnumType(VideoStream_Codec) },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): VideoStream {
    return new VideoStream().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): VideoStream {
    return new VideoStream().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): VideoStream {
    return new VideoStream().fromJsonString(jsonString, options);
  }

  static equals(a: VideoStream | PlainMessage<VideoStream> | undefined, b: VideoStream | PlainMessage<VideoStream> | undefined): boolean {
    return proto3.util.equals(VideoStream, a, b);
  }
}

/**
 * @generated from enum veil.VideoStream.Codec
 */
export enum VideoStream_Codec {
  /**
   * @generated from enum value: CODEC_UNSPECIFIED = 0;
   */
  CODEC_UNSPECIFIED = 0,

  /**
   * @generated from enum value: AVC = 1;
   */
  AVC = 1,

  /**
   * @generated from enum value: HEVC = 2;
   */
  HEVC = 2,

  /**
   * @generated from enum value: AV1 = 3;
   */
  AV1 = 3,

  /**
   * @generated from enum value: VP8 = 4;
   */
  VP8 = 4,

  /**
   * @generated from enum value: VP9 = 5;
   */
  VP9 = 5,
}
// Retrieve enum metadata with: proto3.getEnumType(VideoStream_Codec)
proto3.util.setEnumType(VideoStream_Codec, "veil.VideoStream.Codec", [
  { no: 0, name: "CODEC_UNSPECIFIED" },
  { no: 1, name: "AVC" },
  { no: 2, name: "HEVC" },
  { no: 3, name: "AV1" },
  { no: 4, name: "VP8" },
  { no: 5, name: "VP9" },
]);

/**
 * @generated from message veil.AudioStream
 */
export class AudioStream extends Message<AudioStream> {
  /**
   * @generated from field: veil.AudioStream.Codec encoding = 1;
   */
  encoding = AudioStream_Codec.CODEC_UNSPECIFIED;

  /**
   * @generated from field: repeated veil.AudioStream.Channel channels = 2;
   */
  channels: AudioStream_Channel[] = [];

  constructor(data?: PartialMessage<AudioStream>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.AudioStream";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "encoding", kind: "enum", T: proto3.getEnumType(AudioStream_Codec) },
    { no: 2, name: "channels", kind: "enum", T: proto3.getEnumType(AudioStream_Channel), repeated: true },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): AudioStream {
    return new AudioStream().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): AudioStream {
    return new AudioStream().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): AudioStream {
    return new AudioStream().fromJsonString(jsonString, options);
  }

  static equals(a: AudioStream | PlainMessage<AudioStream> | undefined, b: AudioStream | PlainMessage<AudioStream> | undefined): boolean {
    return proto3.util.equals(AudioStream, a, b);
  }
}

/**
 * @generated from enum veil.AudioStream.Channel
 */
export enum AudioStream_Channel {
  /**
   * @generated from enum value: CHANNEL_UNSPECIFIED = 0;
   */
  CHANNEL_UNSPECIFIED = 0,

  /**
   * @generated from enum value: MONO = 1;
   */
  MONO = 1,

  /**
   * @generated from enum value: FRONT_LEFT = 2;
   */
  FRONT_LEFT = 2,

  /**
   * @generated from enum value: FRONT_RIGHT = 3;
   */
  FRONT_RIGHT = 3,

  /**
   * @generated from enum value: FRONT_CENTER = 4;
   */
  FRONT_CENTER = 4,

  /**
   * @generated from enum value: REAR_CENTER = 5;
   */
  REAR_CENTER = 5,

  /**
   * @generated from enum value: REAR_LEFT = 6;
   */
  REAR_LEFT = 6,

  /**
   * @generated from enum value: REAR_RIGHT = 7;
   */
  REAR_RIGHT = 7,

  /**
   * @generated from enum value: LFE = 8;
   */
  LFE = 8,

  /**
   * @generated from enum value: FRONT_LEFT_OF_CENTER = 9;
   */
  FRONT_LEFT_OF_CENTER = 9,

  /**
   * @generated from enum value: FRONT_RIGHT_OF_CENTER = 10;
   */
  FRONT_RIGHT_OF_CENTER = 10,

  /**
   * @generated from enum value: SIDE_LEFT = 11;
   */
  SIDE_LEFT = 11,

  /**
   * @generated from enum value: SIDE_RIGHT = 12;
   */
  SIDE_RIGHT = 12,
}
// Retrieve enum metadata with: proto3.getEnumType(AudioStream_Channel)
proto3.util.setEnumType(AudioStream_Channel, "veil.AudioStream.Channel", [
  { no: 0, name: "CHANNEL_UNSPECIFIED" },
  { no: 1, name: "MONO" },
  { no: 2, name: "FRONT_LEFT" },
  { no: 3, name: "FRONT_RIGHT" },
  { no: 4, name: "FRONT_CENTER" },
  { no: 5, name: "REAR_CENTER" },
  { no: 6, name: "REAR_LEFT" },
  { no: 7, name: "REAR_RIGHT" },
  { no: 8, name: "LFE" },
  { no: 9, name: "FRONT_LEFT_OF_CENTER" },
  { no: 10, name: "FRONT_RIGHT_OF_CENTER" },
  { no: 11, name: "SIDE_LEFT" },
  { no: 12, name: "SIDE_RIGHT" },
]);

/**
 * @generated from enum veil.AudioStream.Codec
 */
export enum AudioStream_Codec {
  /**
   * @generated from enum value: CODEC_UNSPECIFIED = 0;
   */
  CODEC_UNSPECIFIED = 0,

  /**
   * @generated from enum value: OPUS = 1;
   */
  OPUS = 1,
}
// Retrieve enum metadata with: proto3.getEnumType(AudioStream_Codec)
proto3.util.setEnumType(AudioStream_Codec, "veil.AudioStream.Codec", [
  { no: 0, name: "CODEC_UNSPECIFIED" },
  { no: 1, name: "OPUS" },
]);

/**
 * @generated from message veil.Gamepad
 */
export class Gamepad extends Message<Gamepad> {
  /**
   * @generated from field: string id = 1;
   */
  id = "";

  /**
   * @generated from field: uint32 index = 2;
   */
  index = 0;

  /**
   * @generated from field: veil.Gamepad.Mapping mapping = 3;
   */
  mapping = Gamepad_Mapping.MAPPING_UNSPECIFIED;

  constructor(data?: PartialMessage<Gamepad>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.Gamepad";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "id", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 2, name: "index", kind: "scalar", T: 13 /* ScalarType.UINT32 */ },
    { no: 3, name: "mapping", kind: "enum", T: proto3.getEnumType(Gamepad_Mapping) },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): Gamepad {
    return new Gamepad().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): Gamepad {
    return new Gamepad().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): Gamepad {
    return new Gamepad().fromJsonString(jsonString, options);
  }

  static equals(a: Gamepad | PlainMessage<Gamepad> | undefined, b: Gamepad | PlainMessage<Gamepad> | undefined): boolean {
    return proto3.util.equals(Gamepad, a, b);
  }
}

/**
 * @generated from enum veil.Gamepad.Mapping
 */
export enum Gamepad_Mapping {
  /**
   * @generated from enum value: MAPPING_UNSPECIFIED = 0;
   */
  MAPPING_UNSPECIFIED = 0,

  /**
   * @generated from enum value: STANDARD = 1;
   */
  STANDARD = 1,

  /**
   * @generated from enum value: STANDARD_XR = 2;
   */
  STANDARD_XR = 2,
}
// Retrieve enum metadata with: proto3.getEnumType(Gamepad_Mapping)
proto3.util.setEnumType(Gamepad_Mapping, "veil.Gamepad.Mapping", [
  { no: 0, name: "MAPPING_UNSPECIFIED" },
  { no: 1, name: "STANDARD" },
  { no: 2, name: "STANDARD_XR" },
]);

