import {isTauri} from "../transport";

export async function openUrl(url: string): Promise<void> {
    if (isTauri) {
        const mod = await import("@tauri-apps/plugin-opener");
        await mod.openUrl(url);
    } else {
        window.open(url, "_blank");
    }
}
