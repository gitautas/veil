// @generated by protoc-gen-es v1.10.0 with parameter "target=ts"
// @generated from file video.proto (package veil.video, syntax proto3)
/* eslint-disable */
// @ts-nocheck

import { proto3 } from "@bufbuild/protobuf";

/**
 * @generated from enum veil.video.VideoCodec
 */
export enum VideoCodec {
  /**
   * @generated from enum value: UNSPECIFIED = 0;
   */
  UNSPECIFIED = 0,

  /**
   * @generated from enum value: AVC = 1;
   */
  AVC = 1,

  /**
   * @generated from enum value: HEVC = 2;
   */
  HEVC = 2,
}
// Retrieve enum metadata with: proto3.getEnumType(VideoCodec)
proto3.util.setEnumType(VideoCodec, "veil.video.VideoCodec", [
  { no: 0, name: "UNSPECIFIED" },
  { no: 1, name: "AVC" },
  { no: 2, name: "HEVC" },
]);

