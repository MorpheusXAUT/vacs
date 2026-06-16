export type CallConfig = {
    highlightIncomingCallTarget: boolean;
    enablePriorityCalls: boolean;
    enableCallStartSound: boolean;
    enableCallEndSound: boolean;
    useDefaultCallSources: boolean;
};

export type RemoteConfig = {
    enabled: boolean;
    listenAddr: string;
};

export type RemoteStatus = {
    listening: boolean;
    connectedClients: number;
};

export type ClockMode = "Realtime" | "Relaxed" | "Day";

export const ALL_CPL_MODES = ["Original", "Fast"] as const;
export type CplMode = (typeof ALL_CPL_MODES)[number];

export function isCplMode(value: string): value is CplMode {
    return ALL_CPL_MODES.includes(value as CplMode);
}
