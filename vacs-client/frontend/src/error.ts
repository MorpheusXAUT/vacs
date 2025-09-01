import {invoke, InvokeArgs} from "@tauri-apps/api/core";
import {useErrorOverlayStore} from "./stores/error-overlay-store.ts";
import {error} from "@tauri-apps/plugin-log";

export type Error = {
    title: string;
    message: string;
    timeout_ms?: number;
};

export type CallError = {
    peerId: string;
    reason: string;
}

export async function invokeSafe<T>(cmd: string, args?: InvokeArgs): Promise<T | undefined> {
    try {
        return await invoke<T>(cmd, args);
    } catch(e) {
        openErrorOverlayFromUnknown(e);
    }
}

export async function invokeStrict<T>(cmd: string, args?: InvokeArgs): Promise<T> {
    try {
        return await invoke<T>(cmd, args);
    } catch(e) {
        openErrorOverlayFromUnknown(e);
        throw e;
    }
}

export function openErrorOverlayFromUnknown(e: unknown) {
    const openErrorOverlay = useErrorOverlayStore.getState().open;

    if (isError(e)) {
        openErrorOverlay(e.title, e.message, e.timeout_ms);
    } else {
        void error(JSON.stringify(e));
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