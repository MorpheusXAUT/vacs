import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {ClientInfo} from "../types/client-info.ts";
import {useCallStore} from "../stores/call-store.ts";
import {useErrorOverlayStore} from "../stores/error-overlay-store.ts";
import {useCallListStore} from "../stores/call-list-store.ts";

export function setupSignalingListeners() {
    const {setConnectionState, setDisplayName, setClients, addClient, removeClient} = useSignalingStore.getState();
    const {
        addIncomingCall,
        removePeer,
        rejectPeer,
        acceptCall,
        reset: resetCallStore
    } = useCallStore.getState().actions;
    const {open: openErrorOverlay} = useErrorOverlayStore.getState();
    const {addCall: addCallToCallList, clearCallList} = useCallListStore.getState().actions;

    const unlistenFns: (Promise<UnlistenFn>)[] = [];

    const init = () => {
        unlistenFns.push(
            listen<string>("signaling:connected", (event) => {
                setConnectionState("connected");
                setDisplayName(event.payload);
            }),
            listen("signaling:reconnecting", () => {
                setConnectionState("connecting");
            }),
            listen("signaling:disconnected", () => {
                setConnectionState("disconnected");
                setDisplayName("");
                setClients([]);
                resetCallStore();
                clearCallList();
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
            listen<string>("signaling:call-invite", (event) => {
                addIncomingCall(event.payload);
            }),
            listen<string>("signaling:call-accept", (event) => {
                acceptCall(event.payload);
            }),
            listen<string>("signaling:call-end", (event) => {
                removePeer(event.payload);
            }),
            listen<string>("signaling:call-reject", (event) => {
                rejectPeer(event.payload);
            }),
            listen<string>("signaling:peer-not-found", (event) => {
                removeClient(event.payload);
                removePeer(event.payload);
                openErrorOverlay("Peer not found", `Can not find peer with CID ${event.payload}`, false, 5000);
            }),
            listen<{ incoming: boolean, peerId: string }>("signaling:add-to-call-list", (event) => {
                console.log("add to call list", event.payload);
                addCallToCallList({
                    type: event.payload.incoming ? "IN" : "OUT",
                    time: new Date().toLocaleString('de-AT', {hour: "2-digit", minute: "2-digit"}),
                    name: "LOVV_CTR", // TODO: Update display name
                    number: event.payload.peerId
                });
            })
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    }
}