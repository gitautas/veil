/**
 * SignalingClient - Handles WebRTC signaling with the Veil server
 *
 * This class manages the connection to the signaling server and
 * handles offer/answer exchange and ICE candidate negotiation.
 */

import { SignalingMessage } from './types';

type SignalingEventMap = {
  connected: void;
  disconnected: void;
  offer: string;
  answer: string;
  icecandidate: RTCIceCandidateInit;
  error: Error;
  statechange: string;
};

/**
 * Client for WebRTC signaling communication
 */
export class SignalingClient {
  private url: string;
  private peerId: string;
  private eventSource: EventSource | null = null;
  private connected: boolean = false;

  private eventListeners: Map<
    keyof SignalingEventMap,
    Set<(data: any) => void>
  > = new Map();

  /**
   * Create a new signaling client
   *
   * @param url - Signaling server URL
   * @param peerId - Unique peer identifier
   */
  constructor(url: string, peerId: string) {
    this.url = url;
    this.peerId = peerId;
  }

  /**
   * Connect to the signaling server
   */
  async connect(): Promise<void> {
    if (this.connected) {
      throw new Error('Already connected to signaling server');
    }

    const httpUrl = this.url.replace(/^ws/, 'http');

    try {
      // Connect to SSE endpoint for receiving messages
      this.eventSource = new EventSource(`${httpUrl}/connect/${this.peerId}`);

      this.eventSource.onopen = () => {
        this.connected = true;
        this.emit('connected', undefined);
        console.log('[SignalingClient] Connected to signaling server');
      };

      this.eventSource.onerror = (error) => {
        console.error('[SignalingClient] EventSource error:', error);
        this.connected = false;
        this.emit('error', new Error('Signaling connection error'));
        this.eventSource?.close();
        this.eventSource = null;
      };

      this.eventSource.onmessage = (event) => {
        this.handleMessage(event.data);
      };

      // Wait for connection to be established
      await this.waitForConnection();
    } catch (error) {
      this.connected = false;
      throw new Error(`Failed to connect to signaling server: ${error}`);
    }
  }

  /**
   * Disconnect from the signaling server
   */
  async disconnect(): Promise<void> {
    if (!this.connected) {
      return;
    }

    const httpUrl = this.url.replace(/^ws/, 'http');

    try {
      // Notify server of disconnect
      await fetch(`${httpUrl}/disconnect/${this.peerId}`, {
        method: 'POST',
      });
    } catch (error) {
      console.error('[SignalingClient] Error during disconnect:', error);
    }

    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }

    this.connected = false;
    this.emit('disconnected', undefined);
    console.log('[SignalingClient] Disconnected from signaling server');
  }

  /**
   * Send an SDP offer to the server
   *
   * @param sdp - SDP offer string
   */
  async sendOffer(sdp: string): Promise<void> {
    const httpUrl = this.url.replace(/^ws/, 'http');

    const message: SignalingMessage = {
      type: 'offer',
      sdp,
      peerId: this.peerId,
    };

    const response = await fetch(`${httpUrl}/offer`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(message),
    });

    if (!response.ok) {
      throw new Error(`Failed to send offer: ${response.statusText}`);
    }

    console.log('[SignalingClient] Offer sent');
  }

  /**
   * Send an SDP answer to the server
   *
   * @param sdp - SDP answer string
   */
  async sendAnswer(sdp: string): Promise<void> {
    const httpUrl = this.url.replace(/^ws/, 'http');

    const message: SignalingMessage = {
      type: 'answer',
      sdp,
      peerId: this.peerId,
    };

    const response = await fetch(`${httpUrl}/answer`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(message),
    });

    if (!response.ok) {
      throw new Error(`Failed to send answer: ${response.statusText}`);
    }

    console.log('[SignalingClient] Answer sent');
  }

  /**
   * Send an ICE candidate to the server
   *
   * @param candidate - RTCIceCandidate to send
   */
  async sendIceCandidate(candidate: RTCIceCandidate): Promise<void> {
    const httpUrl = this.url.replace(/^ws/, 'http');

    const message: SignalingMessage = {
      type: 'iceCandidate',
      peerId: this.peerId,
      candidate: candidate.candidate,
      sdpMid: candidate.sdpMid || undefined,
      sdpMLineIndex: candidate.sdpMLineIndex ?? undefined,
    };

    const response = await fetch(`${httpUrl}/ice-candidate`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(message),
    });

    if (!response.ok) {
      throw new Error(`Failed to send ICE candidate: ${response.statusText}`);
    }

    console.log('[SignalingClient] ICE candidate sent');
  }

  /**
   * Handle incoming signaling message
   */
  private handleMessage(data: string): void {
    try {
      const message: SignalingMessage = JSON.parse(data);

      console.log('[SignalingClient] Received message:', message.type);

      switch (message.type) {
        case 'offer':
          this.emit('offer', message.sdp);
          break;
        case 'answer':
          this.emit('answer', message.sdp);
          break;
        case 'iceCandidate':
          this.emit('icecandidate', {
            candidate: message.candidate,
            sdpMid: message.sdpMid,
            sdpMLineIndex: message.sdpMLineIndex,
          });
          break;
        case 'stateChange':
          this.emit('statechange', message.state);
          break;
        case 'error':
          this.emit('error', new Error(message.message));
          break;
        default:
          console.warn('[SignalingClient] Unknown message type:', message);
      }
    } catch (error) {
      console.error('[SignalingClient] Error handling message:', error);
      this.emit('error', error as Error);
    }
  }

  /**
   * Wait for connection to be established
   */
  private waitForConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Connection timeout'));
      }, 10000);

      const checkConnection = () => {
        if (this.connected) {
          clearTimeout(timeout);
          resolve();
        } else if (!this.eventSource) {
          clearTimeout(timeout);
          reject(new Error('Connection failed'));
        } else {
          setTimeout(checkConnection, 100);
        }
      };

      checkConnection();
    });
  }

  /**
   * Add event listener
   */
  on<K extends keyof SignalingEventMap>(
    event: K,
    listener: (data: SignalingEventMap[K]) => void
  ): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event)!.add(listener as any);
  }

  /**
   * Remove event listener
   */
  off<K extends keyof SignalingEventMap>(
    event: K,
    listener: (data: SignalingEventMap[K]) => void
  ): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.delete(listener as any);
    }
  }

  /**
   * Emit event to listeners
   */
  private emit<K extends keyof SignalingEventMap>(
    event: K,
    data: SignalingEventMap[K]
  ): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.forEach((listener) => {
        try {
          listener(data);
        } catch (error) {
          console.error('[SignalingClient] Error in event listener:', error);
        }
      });
    }
  }

  /**
   * Check if connected to signaling server
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Get peer ID
   */
  getPeerId(): string {
    return this.peerId;
  }
}
