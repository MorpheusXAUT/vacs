import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useCallStore} from "../stores/call-store.ts";
import {CallError} from "../error.ts";

export function setupWebrtcListeners() {
    const {errorPeer, setConnectionState} = useCallStore.getState().actions;

    const unlistenFns: Promise<UnlistenFn>[] = [];

    const init = () => {
        unlistenFns.push(
            listen<string>("webrtc:call-connected", event => {
                setConnectionState(event.payload, "connected");
            }),
            listen<string>("webrtc:call-disconnected", event => {
                setConnectionState(event.payload, "disconnected");
            }),
            listen<CallError>("webrtc:call-error", event => {
                errorPeer(event.payload.peerId, event.payload.reason);
            }),
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    };
}
