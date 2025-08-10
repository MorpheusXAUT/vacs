import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {ClientInfo} from "../types/client-info.ts";
import {useCallStore} from "../stores/call-store.ts";
import {CallOffer} from "../types/call.ts";
import {useErrorOverlayStore} from "../stores/error-overlay-store.ts";

export function setupSignalingListeners() {
    const { setConnected, setDisplayName, setClients, addClient, removeClient } = useSignalingStore.getState();
    const { addIncomingCall, removePeer, rejectPeer, acceptCall } = useCallStore.getState().actions;
    const { open: openErrorOverlay } = useErrorOverlayStore.getState();

    const unlistenFns: (Promise<UnlistenFn>)[] = [];

    const init = () => {
        unlistenFns.push(
            listen<string>("signaling:connected", (event) => {
                setConnected(true);
                setDisplayName(event.payload);
            }),
            listen("signaling:disconnected", () => {
                setConnected(false);
                setDisplayName("");
            }),
            listen<ClientInfo[]>("signaling:client-list", (event) => {
                setClients(event.payload);
            }),
            listen<ClientInfo>("signaling:client-connected", (event) => {
                addClient(event.payload);
            }),
            listen<string>("signaling:client-disconnected", (event) => {
                removeClient(event.payload);
                removePeer(event.payload);
            }),
            listen<CallOffer>("signaling:call-offer", (event) => {
                addIncomingCall(event.payload);
            }),
            listen<string>("signaling:call-answer", (event) => {
                acceptCall(event.payload);
            }),
            listen<string>("signaling:call-end", (event) => {
                removePeer(event.payload);
            }),
            listen<string>("signaling:call-reject", (event) => {
                rejectPeer(event.payload);
            }),
            listen<string>("signaling:peer-not-found", (event) => {
                removePeer(event.payload);
                removeClient(event.payload);
                openErrorOverlay("Peer not found", `Can not find peer with CID ${event.payload}`, 5000);
            }),
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    }
}