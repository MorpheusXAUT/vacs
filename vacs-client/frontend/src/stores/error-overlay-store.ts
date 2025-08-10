import {create} from "zustand/react";

type ErrorOverlayState = {
    visible: boolean;
    title: string;
    message: string;
    timeout_id?: number;
    open: (title: string, message: string, timeout?: number) => void;
    close: () => void;
}

const CLOSED_OVERLAY: Omit<ErrorOverlayState, "open" | "close"> = {visible: false, title: "", message: "", timeout_id: undefined};

export const useErrorOverlayStore = create<ErrorOverlayState>()((set, get) => ({
    visible: false,
    title: "",
    message: "",
    timeout_id: undefined,
    open: (title, message, timeout_ms) => {
        const previous_timeout_id = get().timeout_id;
        if (previous_timeout_id !== undefined) {
            clearTimeout(previous_timeout_id);
        }

        const timeout_id = timeout_ms !== undefined
            ? setTimeout(() => set(CLOSED_OVERLAY), timeout_ms)
            : undefined;

        set({visible: true, title, message, timeout_id});
    },
    close: () => {
        const timeout_id = get().timeout_id;
        if (timeout_id !== undefined) {
            clearTimeout(timeout_id);
        }

        set(CLOSED_OVERLAY);
    }
}));