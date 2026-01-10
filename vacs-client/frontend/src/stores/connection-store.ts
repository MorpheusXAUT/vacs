import {create} from "zustand/react";
import {invoke} from "@tauri-apps/api/core";
import {isError, openErrorOverlayFromUnknown} from "../error.ts";
import {useSignalingStore} from "./signaling-store.ts";
import {ClientInfo} from "../types/client-info.ts";

type State = "connecting" | "connected" | "disconnected";

type ConnectionState = {
    connectionState: State;
    displayName: string; // TODO: Replace with position id
    frequency: string;
    setConnectionState: (connectionState: State) => void;
    setConnectionInfo: (info: Omit<ClientInfo, "id">) => void;
};

export const useConnectionStore = create<ConnectionState>()(set => ({
    connectionState: "disconnected",
    displayName: "",
    alias: undefined,
    frequency: "",
    setConnectionState: connectionState => set({connectionState}),
    setConnectionInfo: info => set({...info}),
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
