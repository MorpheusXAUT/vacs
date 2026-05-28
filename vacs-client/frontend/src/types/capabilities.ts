export type Platform =
    | "Windows"
    | "LinuxX11"
    | "LinuxWayland"
    | "LinuxUnknown"
    | "MacOs"
    | "Unknown";

export type Capabilities = {
    alwaysOnTop: boolean;
    keybindListener: boolean;
    keybindEmitter: boolean;
    playback: boolean;
    platform: Platform;
};
