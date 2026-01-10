import {create} from "zustand/react";
import {invoke} from "@tauri-apps/api/core";
import {isError, openErrorOverlayFromUnknown} from "../error.ts";
import {useSignalingStore} from "./signaling-store.ts";

type State = "connecting" | "connected" | "disconnected";

type ConnectionState = {
    connectionState: State;
    setConnectionState: (connectionState: State) => void;
};

export const useConnectionStore = create<ConnectionState>()(set => ({
    connectionState: "disconnected",
    setConnectionState: connectionState => set({connectionState}),
}));

export const connect = async () => {
    const {setConnectionState} = useConnectionStore.getState();
    const {setTerminateOverlayOpen} = useSignalingStore.getState();

    setConnectionState("connecting");
    try {
        await invoke("signaling_connect");
    } catch (e) {
        setConnectionState("disconnected");
        if (
            isError(e) &&
            (e.message === "Login failed: Another client with your CID is already connected." ||
                e.message === "Already connected")
        ) {
            setTerminateOverlayOpen(true);
            return;
        }
        openErrorOverlayFromUnknown(e);
    }
};
