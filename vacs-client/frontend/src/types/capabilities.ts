export type Platform = "Windows" | "LinuxX11" | "LinuxWayland" | "LinuxUnknown" | "MacOs" | "Unknown";

export type Capabilities = {
    alwaysOnTop: boolean;
    keybinds: boolean;
    platform: Platform;
};