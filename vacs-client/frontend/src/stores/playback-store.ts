import {create} from "zustand/react";
import {StateSetter} from "../types/generic.ts";
import {isTauri} from "../transport";
import {instanceId} from "../transport/store-sync.ts";

export type PlaybackStatus = {
    id: number;
    status: "playing" | "paused";
    continuously: boolean;
    progress: number;
};

export type PlaybackDevice = "Output" | "Speaker";

type PlaybackState = {
    selected: number;
    status: PlaybackStatus | undefined;
    playbackDevice: PlaybackDevice;
    openInstanceIds: string[];
    actions: {
        setSelected: StateSetter<PlaybackState["selected"]>;
        setStatus: StateSetter<PlaybackState["status"]>;
        setPlaybackDevice: StateSetter<PlaybackState["playbackDevice"]>;
        setOpenInstanceIds: StateSetter<PlaybackState["openInstanceIds"]>;
    };
};

export const usePlaybackStore = create<PlaybackState>()((set, get) => ({
    selected: 0,
    status: undefined,
    playbackDevice: "Output",
    openInstanceIds: [],
    actions: {
        setSelected: selected => {
            if (typeof selected === "function") {
                selected = selected(get().selected);
            }
            set({selected});
        },
        setStatus: status => {
            if (typeof status === "function") {
                status = status(get().status);
            }
            set({status});
        },
        setPlaybackDevice: device => {
            if (typeof device === "function") {
                device = device(get().playbackDevice);
            }
            set({playbackDevice: device});
        },
        setOpenInstanceIds: openInstanceIds => {
            if (typeof openInstanceIds === "function") {
                openInstanceIds = openInstanceIds(get().openInstanceIds);
            }
            set({openInstanceIds});
        },
    },
}));

export const isPlaybackPaused = () => usePlaybackStore.getState().status?.status === "paused";

export const isPlaybackRoot = () => {
    const openInstanceIds = usePlaybackStore.getState().openInstanceIds.sort();
    if (openInstanceIds.length === 0) return isTauri;
    return openInstanceIds[0] === instanceId;
};
