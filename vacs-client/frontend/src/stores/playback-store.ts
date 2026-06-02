import {create} from "zustand/react";
import {createSetter, StateSetter} from "../types/generic.ts";
import {isTauri} from "../transport";
import {instanceId} from "../transport/store-sync.ts";
import {shouldStopBlinking, startBlink, stopBlink} from "./blink-store.ts";
import {useCallStore} from "./call-store.ts";
import {useRadioStore} from "./radio-store.ts";

export type PlaybackStatus = {
    id: number;
    status: "playing" | "paused";
    continuously: boolean;
    progress: number;
};

export type PlaybackStatusBase = Pick<PlaybackStatus, "id" | "status" | "continuously">;

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

export const usePlaybackStore = create<PlaybackState>()(set => {
    const setter = createSetter<PlaybackState>(set);
    return {
        selected: 0,
        status: undefined,
        playbackDevice: "Output",
        openInstanceIds: [],
        actions: {
            setSelected: setter("selected"),
            setStatus: setter("status", (next, prev) => {
                if (prev?.status !== "paused" && next?.status === "paused") {
                    startBlink();
                } else if (
                    prev?.status === "paused" &&
                    next?.status !== "paused" &&
                    shouldStopBlinking(
                        useCallStore.getState().incomingCalls.length,
                        useCallStore.getState().callDisplay,
                        useRadioStore.getState().cpl,
                        false,
                    )
                ) {
                    stopBlink();
                }
            }),
            setPlaybackDevice: setter("playbackDevice"),
            setOpenInstanceIds: setter("openInstanceIds"),
        },
    };
});

export const isPlaybackPaused = () => usePlaybackStore.getState().status?.status === "paused";

export const isPlaybackRoot = () => {
    const openInstanceIds = [...usePlaybackStore.getState().openInstanceIds].sort();
    if (openInstanceIds.length === 0) return isTauri;
    return openInstanceIds[0] === instanceId;
};
