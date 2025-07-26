import {invoke, InvokeArgs} from "@tauri-apps/api/core";
import {useErrorOverlayStore} from "./stores/error-overlay-store.ts";

export type Error = {
    title: string;
    message: string;
    timeout_ms?: number;
};

export async function invokeWithErrorOverlay(cmd: string, args?: InvokeArgs) {
    try {
        await invoke(cmd, args);
    } catch(e) {
        openErrorOverlayFromUnknown(e);
    }
}

export function openErrorOverlayFromUnknown(e: unknown) {
    const openErrorOverlay = useErrorOverlayStore.getState().open;

    if (isError(e)) {
        openErrorOverlay(e.title, e.message, e.timeout_ms);
    } else {
        console.error(e);
        openErrorOverlay("Unexpected error", "An unknown error occurred");
    }
}

export function isError(err: unknown): err is Error {
    return (
        typeof err === 'object' &&
        err !== null &&
        typeof (err as any).title === 'string' &&
        typeof (err as any).message === 'string' &&
        (typeof (err as any).timeout_ms === 'undefined' ||
            typeof (err as any).timeout_ms === 'number')
    );
}