import {create} from "zustand/react";
import {StateSetter} from "../types/generic.ts";

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
    actions: {
        setSelected: StateSetter<PlaybackState["selected"]>;
        setStatus: StateSetter<PlaybackState["status"]>;
        setPlaybackDevice: StateSetter<PlaybackState["playbackDevice"]>;
    };
};

export const usePlaybackStore = create<PlaybackState>()((set, get) => ({
    selected: 0,
    status: undefined,
    playbackDevice: "Output",
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
    },
}));

export const isPlaybackPaused = () => usePlaybackStore.getState().status?.status === "paused";
