// @generated by protoc-gen-es v1.10.0 with parameter "target=ts"
// @generated from file webrtc.proto (package veil.webrtc, syntax proto3)
/* eslint-disable */
// @ts-nocheck

import type { BinaryReadOptions, FieldList, JsonReadOptions, JsonValue, PartialMessage, PlainMessage } from "@bufbuild/protobuf";
import { Message, proto3 } from "@bufbuild/protobuf";

/**
 * @generated from message veil.webrtc.SessionDescription
 */
export class SessionDescription extends Message<SessionDescription> {
  /**
   * @generated from field: veil.webrtc.SessionDescription.Type type = 1;
   */
  type = SessionDescription_Type.SDP_UNSPECIFIED;

  /**
   * @generated from field: string sdp = 2;
   */
  sdp = "";

  constructor(data?: PartialMessage<SessionDescription>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.webrtc.SessionDescription";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "type", kind: "enum", T: proto3.getEnumType(SessionDescription_Type) },
    { no: 2, name: "sdp", kind: "scalar", T: 9 /* ScalarType.STRING */ },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): SessionDescription {
    return new SessionDescription().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): SessionDescription {
    return new SessionDescription().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): SessionDescription {
    return new SessionDescription().fromJsonString(jsonString, options);
  }

  static equals(a: SessionDescription | PlainMessage<SessionDescription> | undefined, b: SessionDescription | PlainMessage<SessionDescription> | undefined): boolean {
    return proto3.util.equals(SessionDescription, a, b);
  }
}

/**
 * @generated from enum veil.webrtc.SessionDescription.Type
 */
export enum SessionDescription_Type {
  /**
   * @generated from enum value: SDP_UNSPECIFIED = 0;
   */
  SDP_UNSPECIFIED = 0,

  /**
   * @generated from enum value: OFFER = 1;
   */
  OFFER = 1,

  /**
   * @generated from enum value: ANSWER = 2;
   */
  ANSWER = 2,
}
// Retrieve enum metadata with: proto3.getEnumType(SessionDescription_Type)
proto3.util.setEnumType(SessionDescription_Type, "veil.webrtc.SessionDescription.Type", [
  { no: 0, name: "SDP_UNSPECIFIED" },
  { no: 1, name: "OFFER" },
  { no: 2, name: "ANSWER" },
]);

/**
 * @generated from message veil.webrtc.IceCandidate
 */
export class IceCandidate extends Message<IceCandidate> {
  /**
   * @generated from field: string candidate = 1;
   */
  candidate = "";

  /**
   * @generated from field: string sdp_mid = 2;
   */
  sdpMid = "";

  /**
   * @generated from field: uint32 sdp_mline_index = 3;
   */
  sdpMlineIndex = 0;

  /**
   * @generated from field: string username_fragment = 4;
   */
  usernameFragment = "";

  constructor(data?: PartialMessage<IceCandidate>) {
    super();
    proto3.util.initPartial(data, this);
  }

  static readonly runtime: typeof proto3 = proto3;
  static readonly typeName = "veil.webrtc.IceCandidate";
  static readonly fields: FieldList = proto3.util.newFieldList(() => [
    { no: 1, name: "candidate", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 2, name: "sdp_mid", kind: "scalar", T: 9 /* ScalarType.STRING */ },
    { no: 3, name: "sdp_mline_index", kind: "scalar", T: 13 /* ScalarType.UINT32 */ },
    { no: 4, name: "username_fragment", kind: "scalar", T: 9 /* ScalarType.STRING */ },
  ]);

  static fromBinary(bytes: Uint8Array, options?: Partial<BinaryReadOptions>): IceCandidate {
    return new IceCandidate().fromBinary(bytes, options);
  }

  static fromJson(jsonValue: JsonValue, options?: Partial<JsonReadOptions>): IceCandidate {
    return new IceCandidate().fromJson(jsonValue, options);
  }

  static fromJsonString(jsonString: string, options?: Partial<JsonReadOptions>): IceCandidate {
    return new IceCandidate().fromJsonString(jsonString, options);
  }

  static equals(a: IceCandidate | PlainMessage<IceCandidate> | undefined, b: IceCandidate | PlainMessage<IceCandidate> | undefined): boolean {
    return proto3.util.equals(IceCandidate, a, b);
  }
}

