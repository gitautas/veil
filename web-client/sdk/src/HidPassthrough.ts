/**
 * HID passthrough: devices opened with WebHID are recreated on the server
 * (with uhid), so games and drivers there see the real device: a DualSense
 * with its touchpad, motion sensors, lightbar and PS button, a racing wheel,
 * a flight stick, and so on.
 *
 * WebHID only exposes a parsed view of a device's report descriptor, so the
 * descriptor is rebuilt from it here. Browsers keep keyboards and mice from
 * WebHID (and the server refuses them), so those stay on ordinary key and
 * mouse events.
 */

import { HID_OPEN, HID_CLOSE, HID_REPLY, INPUT_HID_REPORT } from './constants';
import type { HidPassthroughInfo } from './types';

// ─── The parts of WebHID we use (not in the DOM typings) ─────────────────────

export interface HidReportItem {
  isAbsolute: boolean;
  isArray: boolean;
  isBufferedBytes: boolean;
  isConstant: boolean;
  isLinear: boolean;
  isRange: boolean;
  isVolatile: boolean;
  hasNull: boolean;
  hasPreferredState: boolean;
  wrap: boolean;
  /** Extended usages: page << 16 | usage. */
  usages: number[];
  usageMinimum: number;
  usageMaximum: number;
  reportSize: number;
  reportCount: number;
  unitExponent: number;
  unitSystem: string;
  unitFactorLengthExponent: number;
  unitFactorMassExponent: number;
  unitFactorTimeExponent: number;
  unitFactorTemperatureExponent: number;
  unitFactorCurrentExponent: number;
  unitFactorLuminousIntensityExponent: number;
  logicalMinimum: number;
  logicalMaximum: number;
  physicalMinimum: number;
  physicalMaximum: number;
}

export interface HidReportInfo {
  reportId: number;
  items: HidReportItem[];
}

export interface HidCollectionInfo {
  usagePage: number;
  usage: number;
  type: number;
  children: HidCollectionInfo[];
  inputReports: HidReportInfo[];
  outputReports: HidReportInfo[];
  featureReports: HidReportInfo[];
}

export interface HidDevice extends EventTarget {
  opened: boolean;
  vendorId: number;
  productId: number;
  productName: string;
  collections: HidCollectionInfo[];
  open(): Promise<void>;
  close(): Promise<void>;
  sendReport(reportId: number, data: BufferSource): Promise<void>;
  sendFeatureReport(reportId: number, data: BufferSource): Promise<void>;
  receiveFeatureReport(reportId: number): Promise<DataView>;
}

interface HidInputReportEvent extends Event {
  device: HidDevice;
  reportId: number;
  data: DataView;
}

// ─── Rebuilding the report descriptor ───────────────────────────────────────

const UNIT_SYSTEMS: Record<string, number> = {
  'none': 0, 'si-linear': 1, 'si-rotation': 2, 'english-linear': 3, 'english-rotation': 4, 'vendor-defined': 0xf,
};

/** A short item's bytes, with the smallest data size that holds `value`. */
function shortItem(prefix: number, value: number, signed: boolean): number[] {
  const fits = (bytes: number) => {
    const bits = bytes * 8;
    return signed
      ? value >= -(2 ** (bits - 1)) && value < 2 ** (bits - 1)
      : value >= 0 && value < 2 ** bits;
  };
  const size = value === 0 ? 0 : fits(1) ? 1 : fits(2) ? 2 : 4;
  const code = size === 4 ? 3 : size;
  const out = [prefix | code];
  for (let i = 0; i < size; i++) out.push((value >>> (8 * i)) & 0xff);
  return out;
}

function unitCode(item: HidReportItem): number {
  const system = UNIT_SYSTEMS[item.unitSystem] ?? 0;
  if (system === 0) return 0;
  const nib = (v: number) => v & 0xf;
  return (system
    | nib(item.unitFactorLengthExponent) << 4
    | nib(item.unitFactorMassExponent) << 8
    | nib(item.unitFactorTimeExponent) << 12
    | nib(item.unitFactorTemperatureExponent) << 16
    | nib(item.unitFactorCurrentExponent) << 20
    | nib(item.unitFactorLuminousIntensityExponent) << 24) >>> 0;
}

/**
 * Rebuild a HID report descriptor from WebHID's collections. Report IDs,
 * field order and sizes are preserved, which is what drivers and games rely
 * on; where a report mixes items from a collection and its sub-collections,
 * the collection's own items come first.
 */
export function buildReportDescriptor(collections: HidCollectionInfo[]): Uint8Array {
  const out: number[] = [];
  // Global state, so items are only emitted when they change.
  const g = new Map<number, number>();
  const global = (prefix: number, value: number, signed = false) => {
    if (g.get(prefix) === value) return;
    g.set(prefix, value);
    out.push(...shortItem(prefix, value, signed));
  };
  const usagePage = (page: number) => global(0x04, page);

  const emitItem = (main: number, item: HidReportItem) => {
    // Usages (local): ranges, or a list. All in one page, or extended.
    const usages = item.isRange ? [item.usageMinimum, item.usageMaximum] : item.usages;
    const pages = new Set(usages.map((u) => u >>> 16));
    const extended = pages.size > 1;
    if (usages.length > 0 && !extended) usagePage(usages[0]! >>> 16);
    const usage = (prefix: number, u: number) =>
      out.push(...(extended ? shortItemFixed4(prefix, u) : shortItem(prefix, u & 0xffff, false)));
    if (item.isRange) {
      usage(0x18, item.usageMinimum);
      usage(0x28, item.usageMaximum);
    } else {
      for (const u of item.usages) usage(0x08, u);
    }

    global(0x14, item.logicalMinimum, true);
    global(0x24, item.logicalMaximum, true);
    global(0x34, item.physicalMinimum, true);
    global(0x44, item.physicalMaximum, true);
    global(0x54, item.unitExponent & 0xf);
    global(0x64, unitCode(item));
    global(0x74, item.reportSize);
    global(0x94, item.reportCount);

    const flags = (item.isConstant ? 1 : 0)
      | (item.isArray ? 0 : 1 << 1)
      | (item.isAbsolute ? 0 : 1 << 2)
      | (item.wrap ? 1 << 3 : 0)
      | (item.isLinear ? 0 : 1 << 4)
      | (item.hasPreferredState ? 0 : 1 << 5)
      | (item.hasNull ? 1 << 6 : 0)
      | (item.isVolatile && main !== 0x80 ? 1 << 7 : 0)
      | (item.isBufferedBytes ? 1 << 8 : 0);
    // Buffered bytes needs the 2-byte form; otherwise the shortest.
    out.push(...(flags > 0xff ? [main | 2, flags & 0xff, flags >> 8] : shortItem(main, flags, false)));
  };

  const emitReports = (main: number, reports: HidReportInfo[]) => {
    for (const report of reports) {
      if (report.reportId !== 0) global(0x84, report.reportId);
      for (const item of report.items) emitItem(main, item);
    }
  };

  const emitCollection = (c: HidCollectionInfo) => {
    usagePage(c.usagePage);
    out.push(...shortItem(0x08, c.usage, false));
    out.push(...shortItem(0xa0, c.type, false));
    emitReports(0x80, c.inputReports);
    emitReports(0x90, c.outputReports);
    emitReports(0xb0, c.featureReports);
    for (const child of c.children) emitCollection(child);
    out.push(0xc0);
  };

  for (const c of collections) emitCollection(c);
  return new Uint8Array(out);
}

/** A 4-byte short item (extended usages always take 4 bytes). */
function shortItemFixed4(prefix: number, value: number): number[] {
  return [prefix | 3, value & 0xff, (value >>> 8) & 0xff, (value >>> 16) & 0xff, (value >>> 24) & 0xff];
}

/** Whether any report has an ID (then every report starts with one). */
export function usesReportIds(collections: HidCollectionInfo[]): boolean {
  return collections.some((c) =>
    [...c.inputReports, ...c.outputReports, ...c.featureReports].some((r) => r.reportId !== 0)
    || usesReportIds(c.children));
}

function inputReportIds(collections: HidCollectionInfo[]): Set<number> {
  const ids = new Set<number>();
  const walk = (c: HidCollectionInfo) => {
    c.inputReports.forEach((r) => ids.add(r.reportId));
    c.children.forEach(walk);
  };
  collections.forEach(walk);
  return ids;
}

/**
 * WebHID doesn't say how a device is connected, but drivers parse some
 * devices' reports differently over Bluetooth. Sony pads are recognisable by
 * their Bluetooth-only input reports (0x31 DualSense, 0x11 DualShock 4).
 */
export function guessBus(device: Pick<HidDevice, 'vendorId' | 'collections'>): 'usb' | 'bluetooth' {
  if (device.vendorId === 0x054c) {
    const ids = inputReportIds(device.collections);
    if (ids.has(0x31) || ids.has(0x11)) return 'bluetooth';
  }
  return 'usb';
}

// ─── Relaying reports ───────────────────────────────────────────────────────

interface Passed {
  info: HidPassthroughInfo;
  device: HidDevice;
  usesIds: boolean;
  onInput: (e: Event) => void;
}

/** How the passthrough talks to the server. */
export interface HidTransport {
  /** Send reliably (a stream). */
  stream(data: Uint8Array): Promise<void>;
  /** Send fast, possibly lost (a datagram). */
  datagram(data: Uint8Array): void;
  log(...args: unknown[]): void;
  changed(list: HidPassthroughInfo[]): void;
}

export class HidPassthrough {
  private devices = new Map<number, Passed>();
  private nextHandle = 0;

  constructor(private transport: HidTransport) {}

  list(): HidPassthroughInfo[] {
    return [...this.devices.values()].map((d) => ({ ...d.info }));
  }

  /**
   * Whether a Gamepad API pad is (probably) one of the passed-through
   * devices: by vendor/product when its ID shows them, otherwise by name
   * (browsers describe the same pad differently per OS and connection).
   */
  coversPad(padId: string, ids: [number, number] | null): boolean {
    const name = padId.toLowerCase();
    return [...this.devices.values()].some((d) =>
      ids
        ? d.device.vendorId === ids[0] && d.device.productId === ids[1]
        : !!d.device.productName && name.includes(d.device.productName.toLowerCase()));
  }

  /** Open `device` and recreate it on the server. Resolves to its handle. */
  async add(device: HidDevice): Promise<number> {
    for (const d of this.devices.values()) {
      if (d.device === device) return d.info.handle;
    }
    const handle = this.nextHandle++ & 0xff;
    if (!device.opened) await device.open();

    const usesIds = usesReportIds(device.collections);
    const descriptor = buildReportDescriptor(device.collections);
    const info: HidPassthroughInfo = {
      handle,
      name: device.productName || `HID ${hex4(device.vendorId)}:${hex4(device.productId)}`,
      vendorId: device.vendorId,
      productId: device.productId,
      bus: guessBus(device),
      status: 'opening',
      error: null,
    };

    const onInput = (e: Event) => {
      const { reportId, data } = e as HidInputReportEvent;
      const prefix = usesIds ? 1 : 0;
      const buf = new Uint8Array(2 + prefix + data.byteLength);
      buf[0] = INPUT_HID_REPORT;
      buf[1] = handle;
      if (usesIds) buf[2] = reportId;
      buf.set(new Uint8Array(data.buffer, data.byteOffset, data.byteLength), 2 + prefix);
      this.transport.datagram(buf);
    };
    device.addEventListener('inputreport', onInput);
    this.devices.set(handle, { info, device, usesIds, onInput });
    this.transport.changed(this.list());

    const json = JSON.stringify({
      handle,
      name: info.name,
      vendor: device.vendorId,
      product: device.productId,
      bus: info.bus,
      descriptor: Array.from(descriptor),
    });
    const body = new TextEncoder().encode(json);
    const msg = new Uint8Array(1 + body.length);
    msg[0] = HID_OPEN;
    msg.set(body, 1);
    await this.transport.stream(msg);
    this.transport.log(`HID passthrough: opening ${info.name} (${descriptor.length}-byte descriptor, ${info.bus})`);
    return handle;
  }

  /** Stop passing a device through (and close it). */
  async remove(handle: number): Promise<void> {
    const d = this.devices.get(handle);
    if (!d) return;
    this.devices.delete(handle);
    d.device.removeEventListener('inputreport', d.onInput);
    this.transport.changed(this.list());
    await this.transport.stream(new Uint8Array([HID_CLOSE, handle])).catch(() => {});
    await d.device.close().catch(() => {});
  }

  /** Forget every device (e.g. the connection closed). */
  async clear(): Promise<void> {
    const devices = [...this.devices.values()];
    this.devices.clear();
    for (const d of devices) {
      d.device.removeEventListener('inputreport', d.onInput);
      await d.device.close().catch(() => {});
    }
    this.transport.changed([]);
  }

  /** The server's answer to opening a device (MSG 0x1E). */
  handleStatus(status: { handle: number; ok: boolean; error: string | null }): void {
    const d = this.devices.get(status.handle);
    if (!d) return;
    d.info.status = status.ok ? 'active' : 'failed';
    d.info.error = status.error;
    this.transport.changed(this.list());
  }

  /** Something the server needs done on a device (MSG 0x1D). */
  async handleRequest(payload: Uint8Array): Promise<void> {
    if (payload.length < 7) return;
    const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
    const handle = payload[0]!;
    const kind = payload[1]!;
    const request = view.getUint32(2);
    const reportNumber = payload[6]!;
    const data = payload.subarray(7);
    const d = this.devices.get(handle);
    if (!d) return;

    // Reports from the kernel start with the report ID when the device uses
    // them; WebHID takes the ID separately. (Copies, which WebHID accepts.)
    const split = (bytes: Uint8Array, id: number): [number, Uint8Array<ArrayBuffer>] =>
      d.usesIds && bytes.length > 0 ? [bytes[0]!, bytes.slice(1)] : [id, bytes.slice()];

    try {
      if (kind === 0) {
        const [id, body] = split(data, 0);
        await d.device.sendReport(id, body);
        return; // Output reports aren't answered.
      }
      if (kind === 1) {
        const view = await d.device.receiveFeatureReport(reportNumber);
        let bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
        // The kernel wants the report ID first.
        if (d.usesIds && reportNumber !== 0 && bytes[0] !== reportNumber) {
          bytes = Uint8Array.from([reportNumber, ...bytes]);
        }
        await this.reply(handle, kind, request, 0, bytes);
      } else {
        const [id, body] = split(data, reportNumber);
        await (kind === 2 ? d.device.sendFeatureReport(id, body) : d.device.sendReport(id, body));
        await this.reply(handle, kind, request, 0, new Uint8Array());
      }
    } catch (err) {
      this.transport.log(`HID passthrough: ${d.info.name} request failed:`, err instanceof Error ? err.message : err);
      if (kind !== 0) await this.reply(handle, kind, request, 5 /* EIO */, new Uint8Array());
    }
  }

  private async reply(handle: number, kind: number, request: number, status: number, data: Uint8Array): Promise<void> {
    const msg = new Uint8Array(8 + data.length);
    const view = new DataView(msg.buffer);
    msg[0] = HID_REPLY;
    msg[1] = handle;
    msg[2] = kind;
    view.setUint32(3, request);
    msg[7] = status;
    msg.set(data, 8);
    await this.transport.stream(msg);
  }
}

const hex4 = (n: number) => n.toString(16).padStart(4, '0');
