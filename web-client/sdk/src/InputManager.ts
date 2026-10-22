/**
 * InputManager - Handles input capture for keyboard, mouse, and gamepad
 *
 * This class captures user input events and converts them to the format
 * expected by the Veil server for remote session control.
 */

import {
  InputEvent,
  KeyboardEvent as VeilKeyboardEvent,
  MouseButtonEvent,
  MouseMoveEvent,
  MouseWheelEvent,
  GamepadButtonEvent,
  GamepadAxisEvent,
} from './types';
import {
  SCAN_CODE_MAP,
  MOUSE_BUTTON_MAP,
  GAMEPAD_BUTTON_MAP,
  GAMEPAD_AXIS_MAP,
  GAMEPAD_DEADZONE,
  GAMEPAD_POLL_INTERVAL,
} from './constants';

type InputEventHandler = (event: InputEvent) => void;

/**
 * Manages input capture from keyboard, mouse, and gamepad
 */
export class InputManager {
  private element: HTMLElement;
  private captureKeyboard: boolean;
  private captureMouse: boolean;
  private captureGamepad: boolean;
  private pointerLockEnabled: boolean;

  private eventHandlers: Set<InputEventHandler> = new Set();
  private boundHandlers: Map<string, EventListener> = new Map();

  private gamepadPollingInterval: number | null = null;
  private lastGamepadStates: Map<number, Gamepad> = new Map();
  private pointerLocked: boolean = false;

  /**
   * Create a new input manager
   *
   * @param element - HTML element to attach input listeners to
   * @param captureKeyboard - Enable keyboard capture
   * @param captureMouse - Enable mouse capture
   * @param captureGamepad - Enable gamepad capture
   * @param pointerLockEnabled - Enable pointer lock on click
   */
  constructor(
    element: HTMLElement,
    captureKeyboard: boolean = true,
    captureMouse: boolean = true,
    captureGamepad: boolean = true,
    pointerLockEnabled: boolean = false
  ) {
    this.element = element;
    this.captureKeyboard = captureKeyboard;
    this.captureMouse = captureMouse;
    this.captureGamepad = captureGamepad;
    this.pointerLockEnabled = pointerLockEnabled;

    this.setupEventListeners();
  }

  /**
   * Add input event handler
   */
  on(event: 'input', handler: InputEventHandler): void {
    this.eventHandlers.add(handler);
  }

  /**
   * Remove input event handler
   */
  off(event: 'input', handler: InputEventHandler): void {
    this.eventHandlers.delete(handler);
  }

  /**
   * Stop input capture and clean up
   */
  stop(): void {
    this.removeEventListeners();

    if (this.gamepadPollingInterval !== null) {
      clearInterval(this.gamepadPollingInterval);
      this.gamepadPollingInterval = null;
    }

    if (this.pointerLocked) {
      document.exitPointerLock();
      this.pointerLocked = false;
    }
  }

  /**
   * Request pointer lock
   */
  async requestPointerLock(): Promise<void> {
    try {
      await this.element.requestPointerLock();
      this.pointerLocked = true;
    } catch (error) {
      console.error('[InputManager] Failed to request pointer lock:', error);
    }
  }

  /**
   * Exit pointer lock
   */
  exitPointerLock(): void {
    if (this.pointerLocked) {
      document.exitPointerLock();
      this.pointerLocked = false;
    }
  }

  /**
   * Check if pointer is locked
   */
  isPointerLocked(): boolean {
    return this.pointerLocked;
  }

  /**
   * Set up event listeners for input capture
   */
  private setupEventListeners(): void {
    if (this.captureKeyboard) {
      this.setupKeyboardListeners();
    }

    if (this.captureMouse) {
      this.setupMouseListeners();
    }

    if (this.captureGamepad) {
      this.setupGamepadListeners();
    }

    // Pointer lock change listener
    if (this.pointerLockEnabled) {
      const pointerLockChangeHandler = () => {
        this.pointerLocked = document.pointerLockElement === this.element;
      };
      document.addEventListener('pointerlockchange', pointerLockChangeHandler);
      this.boundHandlers.set('pointerlockchange', pointerLockChangeHandler);
    }
  }

  /**
   * Remove all event listeners
   */
  private removeEventListeners(): void {
    this.boundHandlers.forEach((handler, event) => {
      if (event.startsWith('document:')) {
        document.removeEventListener(event.replace('document:', ''), handler);
      } else {
        this.element.removeEventListener(event, handler);
      }
    });
    this.boundHandlers.clear();
  }

  /**
   * Set up keyboard event listeners
   */
  private setupKeyboardListeners(): void {
    const keyDownHandler = (e: Event) => {
      const event = e as KeyboardEvent;
      this.handleKeyboardEvent('keydown', event);
      event.preventDefault();
    };

    const keyUpHandler = (e: Event) => {
      const event = e as KeyboardEvent;
      this.handleKeyboardEvent('keyup', event);
      event.preventDefault();
    };

    this.element.addEventListener('keydown', keyDownHandler);
    this.element.addEventListener('keyup', keyUpHandler);

    this.boundHandlers.set('keydown', keyDownHandler);
    this.boundHandlers.set('keyup', keyUpHandler);

    // Make element focusable
    if (!this.element.hasAttribute('tabindex')) {
      this.element.setAttribute('tabindex', '0');
    }
  }

  /**
   * Set up mouse event listeners
   */
  private setupMouseListeners(): void {
    const mouseDownHandler = (e: Event) => {
      const event = e as MouseEvent;
      this.handleMouseButton('mousedown', event);
      event.preventDefault();

      // Request pointer lock on click if enabled
      if (this.pointerLockEnabled && !this.pointerLocked) {
        this.requestPointerLock();
      }
    };

    const mouseUpHandler = (e: Event) => {
      const event = e as MouseEvent;
      this.handleMouseButton('mouseup', event);
      event.preventDefault();
    };

    const mouseMoveHandler = (e: Event) => {
      const event = e as MouseEvent;
      this.handleMouseMove(event);
    };

    const wheelHandler = (e: Event) => {
      const event = e as WheelEvent;
      this.handleMouseWheel(event);
      event.preventDefault();
    };

    const contextMenuHandler = (e: Event) => {
      e.preventDefault();
    };

    this.element.addEventListener('mousedown', mouseDownHandler);
    this.element.addEventListener('mouseup', mouseUpHandler);
    this.element.addEventListener('mousemove', mouseMoveHandler);
    this.element.addEventListener('wheel', wheelHandler);
    this.element.addEventListener('contextmenu', contextMenuHandler);

    this.boundHandlers.set('mousedown', mouseDownHandler);
    this.boundHandlers.set('mouseup', mouseUpHandler);
    this.boundHandlers.set('mousemove', mouseMoveHandler);
    this.boundHandlers.set('wheel', wheelHandler);
    this.boundHandlers.set('contextmenu', contextMenuHandler);
  }

  /**
   * Set up gamepad polling
   */
  private setupGamepadListeners(): void {
    this.gamepadPollingInterval = window.setInterval(() => {
      this.pollGamepads();
    }, GAMEPAD_POLL_INTERVAL);

    // Listen for gamepad connection events
    const gamepadConnectedHandler = (e: Event) => {
      const event = e as GamepadEvent;
      console.log('[InputManager] Gamepad connected:', event.gamepad.id);
    };

    const gamepadDisconnectedHandler = (e: Event) => {
      const event = e as GamepadEvent;
      console.log('[InputManager] Gamepad disconnected:', event.gamepad.id);
      this.lastGamepadStates.delete(event.gamepad.index);
    };

    window.addEventListener('gamepadconnected', gamepadConnectedHandler);
    window.addEventListener('gamepaddisconnected', gamepadDisconnectedHandler);

    this.boundHandlers.set('document:gamepadconnected', gamepadConnectedHandler);
    this.boundHandlers.set('document:gamepaddisconnected', gamepadDisconnectedHandler);
  }

  /**
   * Handle keyboard event
   */
  private handleKeyboardEvent(type: 'keydown' | 'keyup', event: KeyboardEvent): void {
    const keyCode = SCAN_CODE_MAP[event.code] || 0;
    const char = event.key.length === 1 ? event.key : undefined;

    const veilEvent: VeilKeyboardEvent = {
      type,
      keyCode,
      key: event.key,
      char,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
    };

    this.emitInputEvent(veilEvent);
  }

  /**
   * Handle mouse button event
   */
  private handleMouseButton(type: 'mousedown' | 'mouseup', event: MouseEvent): void {
    const rect = this.element.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    const veilEvent: MouseButtonEvent = {
      type,
      button: event.button,
      x,
      y,
    };

    this.emitInputEvent(veilEvent);
  }

  /**
   * Handle mouse move event
   */
  private handleMouseMove(event: MouseEvent): void {
    const rect = this.element.getBoundingClientRect();
    let x = event.clientX - rect.left;
    let y = event.clientY - rect.top;

    const veilEvent: MouseMoveEvent = {
      type: 'mousemove',
      x,
      y,
    };

    // Include movement delta if pointer is locked
    if (this.pointerLocked) {
      veilEvent.deltaX = event.movementX;
      veilEvent.deltaY = event.movementY;
    }

    this.emitInputEvent(veilEvent);
  }

  /**
   * Handle mouse wheel event
   */
  private handleMouseWheel(event: WheelEvent): void {
    const veilEvent: MouseWheelEvent = {
      type: 'wheel',
      deltaX: event.deltaX,
      deltaY: event.deltaY,
      deltaMode: event.deltaMode,
    };

    this.emitInputEvent(veilEvent);
  }

  /**
   * Poll connected gamepads for input
   */
  private pollGamepads(): void {
    const gamepads = navigator.getGamepads();

    for (let i = 0; i < gamepads.length; i++) {
      const gamepad = gamepads[i];
      if (!gamepad) continue;

      const lastState = this.lastGamepadStates.get(gamepad.index);

      // Check buttons
      gamepad.buttons.forEach((button, buttonIndex) => {
        const lastButton = lastState?.buttons[buttonIndex];
        const pressed = button.pressed;
        const lastPressed = lastButton?.pressed || false;

        if (pressed !== lastPressed) {
          const veilEvent: GamepadButtonEvent = {
            type: 'gamepadbutton',
            gamepadId: gamepad.index,
            button: buttonIndex,
            pressed,
            value: button.value,
          };
          this.emitInputEvent(veilEvent);
        }
      });

      // Check axes
      gamepad.axes.forEach((value, axisIndex) => {
        const lastValue = lastState?.axes[axisIndex] || 0;

        // Apply deadzone
        const adjustedValue = Math.abs(value) < GAMEPAD_DEADZONE ? 0 : value;
        const adjustedLastValue = Math.abs(lastValue) < GAMEPAD_DEADZONE ? 0 : lastValue;

        if (adjustedValue !== adjustedLastValue) {
          const veilEvent: GamepadAxisEvent = {
            type: 'gamepadaxis',
            gamepadId: gamepad.index,
            axis: axisIndex,
            value: adjustedValue,
          };
          this.emitInputEvent(veilEvent);
        }
      });

      // Store current state
      this.lastGamepadStates.set(gamepad.index, gamepad);
    }
  }

  /**
   * Emit input event to handlers
   */
  private emitInputEvent(event: InputEvent): void {
    this.eventHandlers.forEach((handler) => {
      try {
        handler(event);
      } catch (error) {
        console.error('[InputManager] Error in input event handler:', error);
      }
    });
  }
}
