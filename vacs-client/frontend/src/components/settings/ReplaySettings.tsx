import Checkbox from "../ui/Checkbox.tsx";
import {invokeStrict} from "../../error.ts";
import {isTauri} from "../../transport";
import {TargetedEvent} from "preact";
import {useSettingsStore} from "../../stores/settings-store.ts";

function ReplaySettings() {
    const enabled = useSettingsStore(state => state.replayEnabled);
    const setEnabled = useSettingsStore(state => state.setReplayEnabled);

    const handleToggle = async (e: TargetedEvent<HTMLInputElement>) => {
        const next = e.currentTarget.checked;
        try {
            await invokeStrict("replay_set_enabled", {enabled: next});
            setEnabled(next);
        } catch {
            setEnabled(!next);
        }
    };

    return (
        <div className="w-full flex justify-between items-center">
            <label htmlFor="replay-enabled">Enable radio playback</label>
            <Checkbox
                name="replay-enabled"
                checked={enabled}
                onChange={handleToggle}
                disabled={!isTauri}
            />
        </div>
    );
}

export default ReplaySettings;
