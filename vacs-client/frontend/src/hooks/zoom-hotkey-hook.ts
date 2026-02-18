import {useEffect, useRef} from "preact/hooks";
import {getCurrentWebviewWindow} from "@tauri-apps/api/webviewWindow";

const ZoomFactor = 0.05;

export function useZoomHotkey() {
    const zoomRef = useRef<number>(1);

    const handleZoomKeyDown = async (event: KeyboardEvent) => {
        if (!(event.ctrlKey || event.metaKey) || event.shiftKey) return;

        const key = event.key;
        const code = event.code;

        if (key === "+" || code === "NumpadAdd") {
            await getCurrentWebviewWindow().setZoom(zoomRef.current + ZoomFactor);
            zoomRef.current += ZoomFactor;
        } else if (key === "-" || code === "NumpadSubtract") {
            await getCurrentWebviewWindow().setZoom(zoomRef.current - ZoomFactor);
            zoomRef.current -= ZoomFactor;
        } else if (key === "0" || code === "Digit0") {
            await getCurrentWebviewWindow().setZoom(1);
            zoomRef.current = 1;
        }
    };

    useEffect(() => {
        document.addEventListener("keydown", handleZoomKeyDown);

        return () => {
            document.removeEventListener("keydown", handleZoomKeyDown);
        };
    }, []);
}
