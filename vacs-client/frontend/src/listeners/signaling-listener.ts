import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {ClientInfo} from "../types/client-info.ts";
import {useCallStore} from "../stores/call-store.ts";
import {CallOffer} from "../types/call.ts";

export function setupSignalingListeners() {
    const { setConnected, setDisplayName, setClients, addClient, removeClient } = useSignalingStore.getState();
    const { addIncomingCall, removePeer } = useCallStore.getState();

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
                console.log("client-connected", event.payload);
                addClient(event.payload);
            }),
            listen<string>("signaling:client-disconnected", (event) => {
                console.log("client-disconnected", event.payload);
                removeClient(event.payload);
                removePeer(event.payload);
            }),
            listen<CallOffer>("signaling:call-offer", (event) => {
                console.log("call-offer", event.payload);
                addIncomingCall(event.payload);
            }),
            listen<string>("signaling:call-end", (event) => {
                console.log("call-end", event.payload);
                removePeer(event.payload);
            })
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    }
}