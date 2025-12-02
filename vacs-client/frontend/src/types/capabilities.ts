export type Platform = "Windows" | "LinuxX11" | "LinuxWayland" | "LinuxUnknown" | "MacOs" | "Unknown";

export type Capabilities = {
    alwaysOnTop: boolean;
    windowState: boolean;
    keybindListener: boolean;
    keybindEmitter: boolean;
    platform: Platform;
};