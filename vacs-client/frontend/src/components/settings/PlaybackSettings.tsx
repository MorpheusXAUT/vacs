import Checkbox from "../ui/Checkbox.tsx";
import {invokeStrict} from "../../error.ts";
import {isTauri} from "../../transport";
import {TargetedEvent} from "preact";
import {useSettingsStore} from "../../stores/settings-store.ts";

function PlaybackSettings() {
    const enabled = useSettingsStore(state => state.playbackEnabled);
    const setEnabled = useSettingsStore(state => state.setPlaybackEnabled);

    const handleToggle = async (e: TargetedEvent<HTMLInputElement>) => {
        const next = e.currentTarget.checked;
        try {
            await invokeStrict("playback_set_enabled", {enabled: next});
            setEnabled(next);
        } catch {
            setEnabled(!next);
        }
    };

    return (
        <div className="w-full flex justify-between items-center">
            <label htmlFor="playback-enabled">Enable radio playback</label>
            <Checkbox
                name="playback-enabled"
                checked={enabled}
                onChange={handleToggle}
                disabled={!isTauri}
            />
        </div>
    );
}

export default PlaybackSettings;
