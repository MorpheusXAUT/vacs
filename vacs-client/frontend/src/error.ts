import {invoke, InvokeArgs} from "@tauri-apps/api/core";
import {useErrorOverlayStore} from "./stores/error-overlay-store.ts";
import {error} from "@tauri-apps/plugin-log";

export type Error = {
    title: string;
    message: string;
    isNonCritical: boolean;
    timeoutMs?: number;
};

export type CallError = {
    peerId: string;
    reason: string;
};

export async function invokeSafe<T>(cmd: string, args?: InvokeArgs): Promise<T | undefined> {
    try {
        return await invoke<T>(cmd, args);
    } catch (e) {
        openErrorOverlayFromUnknown(e);
    }
}

export async function invokeStrict<T>(cmd: string, args?: InvokeArgs): Promise<T> {
    try {
        return await invoke<T>(cmd, args);
    } catch (e) {
        openErrorOverlayFromUnknown(e);
        throw e;
    }
}

export function openErrorOverlayFromUnknown(e: unknown) {
    const openErrorOverlay = useErrorOverlayStore.getState().open;

    if (isError(e)) {
        openErrorOverlay(e.title, e.message, false, e.timeoutMs);
    } else {
        void error(JSON.stringify(e));
        openErrorOverlay("Unexpected error", "An unknown error occurred", false);
    }
}

export function isError(err: unknown): err is Error {
    if (typeof err !== "object" || err === null) {
        return false;
    }

    const maybeError = err as Record<string, unknown>;

    return (
        typeof maybeError.title === "string" &&
        typeof maybeError.message === "string" &&
        (maybeError.timeout_ms === undefined || typeof maybeError.timeout_ms === "number")
    );
}
