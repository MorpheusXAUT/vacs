import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {ClientInfo} from "../types/client-info.ts";

export function setupSignalingListeners() {
    const { setConnected, setClients } = useSignalingStore.getState();

    const unlistenFns: (Promise<UnlistenFn>)[] = [];

    const init = () => {
        const unlisten1 = listen<ClientInfo[]>("signaling:client-list", (event) => {
            setConnected(true);
            setClients(event.payload);
        });

        unlistenFns.push(unlisten1);
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    }
}