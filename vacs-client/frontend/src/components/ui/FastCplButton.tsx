import {useRadioStore} from "../../stores/radio-store.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import Button from "./Button.tsx";
import {invokeSafe} from "../../error.ts";

function FastCplButton() {
    const radioState = useRadioStore(state => state.radioState?.state ?? "NotConfigured");
    const radioIntegration = useSettingsStore(state => state.radioConfig?.integration);

    const disabled =
        radioState === "NotConfigured" ||
        radioState === "Disconnected" ||
        radioIntegration !== "TrackAudio";
    const textMuted = radioState === "NotConfigured" || radioIntegration !== "TrackAudio";

    return (
        <Button
            color="cyan"
            onClick={() => invokeSafe("radio_fast_couple")}
            disabled={disabled}
            className={textMuted ? "text-slate-400" : ""}
        >
            <p>
                FAST
                <br />
                CPL
            </p>
        </Button>
    );
}

export default FastCplButton;
