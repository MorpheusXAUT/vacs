import {useErrorOverlayStore} from "../stores/error-overlay-store.ts";
import {listen, UnlistenFn} from "@tauri-apps/api/event";
import {Error} from "../error.ts";

export function setupErrorListener() {
    const openErrorOverlay = useErrorOverlayStore.getState().open;

    const unlistenFns: (Promise<UnlistenFn>)[] = [];

    const init = () => {

        unlistenFns.push(
            listen<Error>("error", (event) => {
                openErrorOverlay(event.payload.title, event.payload.message, event.payload.timeout_ms);
            })
        );
    };

    init();

    return () => {
        unlistenFns.forEach(fn => fn.then(f => f()));
    }
}