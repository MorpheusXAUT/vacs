import {describe, expect, it} from "vitest";
import {
    JoystickButton,
    inputEquals,
    inputToLabel,
    isJoystickButton,
    joystickButtonLabel,
} from "../../src/types/transmit.ts";

const YOKE_B2: JoystickButton = {device: "guid-yoke", button: 2, name: "Yoke"};

describe("isJoystickButton", () => {
    it("identifies joystick buttons", () => {
        expect(isJoystickButton(YOKE_B2)).toBe(true);
        expect(isJoystickButton({device: "guid", button: 0})).toBe(true);
    });

    it("rejects key codes and null", () => {
        expect(isJoystickButton("KeyA")).toBe(false);
        expect(isJoystickButton(null)).toBe(false);
    });
});

describe("inputEquals", () => {
    it("compares key codes as strings", () => {
        expect(inputEquals("KeyA", "KeyA")).toBe(true);
        expect(inputEquals("KeyA", "KeyB")).toBe(false);
    });

    it("handles null bindings", () => {
        expect(inputEquals(null, null)).toBe(true);
        expect(inputEquals("KeyA", null)).toBe(false);
        expect(inputEquals(null, YOKE_B2)).toBe(false);
    });

    it("never equates a key code with a button", () => {
        expect(inputEquals("KeyA", YOKE_B2)).toBe(false);
        expect(inputEquals(YOKE_B2, "KeyA")).toBe(false);
    });

    it("compares buttons by device and index, ignoring the display name", () => {
        expect(inputEquals(YOKE_B2, {device: "guid-yoke", button: 2, name: "Renamed"})).toBe(true);
        expect(inputEquals(YOKE_B2, {device: "guid-yoke", button: 2})).toBe(true);
        expect(inputEquals(YOKE_B2, {device: "guid-yoke", button: 3, name: "Yoke"})).toBe(false);
        // same button index on a different device must not match
        expect(inputEquals(YOKE_B2, {device: "guid-throttle", button: 2, name: "Yoke"})).toBe(
            false,
        );
    });
});

describe("joystickButtonLabel", () => {
    it("uses the device name when known", () => {
        expect(joystickButtonLabel(YOKE_B2)).toBe("Yoke B2");
    });

    it("falls back to a generic label", () => {
        expect(joystickButtonLabel({device: "guid", button: 5})).toBe("Joystick B5");
        expect(joystickButtonLabel({device: "guid", button: 5, name: null})).toBe("Joystick B5");
    });
});

describe("inputToLabel", () => {
    it("labels buttons and key codes", async () => {
        expect(await inputToLabel(YOKE_B2)).toBe("Yoke B2");
        // jsdom has no navigator.keyboard, so this exercises the
        // prettyFormatKeyCode fallback
        expect(await inputToLabel("KeyA")).toBe("A");
        expect(await inputToLabel("F13")).toBe("F13");
    });
});
