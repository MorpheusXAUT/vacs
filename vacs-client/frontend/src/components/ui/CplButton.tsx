import {clsx} from "clsx";
import {invokeSafe} from "../../error.ts";
import {useBlinkStore} from "../../stores/blink-store.ts";
import {useRadioStore} from "../../stores/radio-store.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import Button from "./Button.tsx";

function CplButton() {
    const blink = useBlinkStore(state => state.blink);
    const cpl = useRadioStore(state => state.cpl);
    const setCpl = useRadioStore(state => state.setCpl);
    const radioState = useRadioStore(state => state.radioState?.state ?? "NotConfigured");
    const radioIntegration = useSettingsStore(state => state.radioConfig?.integration);

    const disabled =
        radioState === "NotConfigured" ||
        radioState === "Disconnected" ||
        radioIntegration !== "TrackAudio";
    const textMuted = radioState === "NotConfigured" || radioIntegration !== "TrackAudio";

    const handleDoubleClick = () => {
        if (cpl) return;
        void invokeSafe("radio_fast_couple");
    };

    return (
        <Button
            color={cpl ? (blink ? "blue" : "cyan") : "cyan"}
            className={clsx("text-lg", textMuted && "text-gray-500")}
            onClick={() => setCpl(!cpl)}
            onDoubleClick={handleDoubleClick}
            disabled={disabled}
        >
            CPL
        </Button>
    );
}

export default CplButton;
