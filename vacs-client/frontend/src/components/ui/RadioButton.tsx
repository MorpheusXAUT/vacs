import {clsx} from "clsx";
import {invokeStrict} from "../../error.ts";
import {goToPage} from "../../stores/navigation-store.ts";
import {useProfileType} from "../../stores/profile-store.ts";
import {useRadioStore} from "../../stores/radio-store.ts";
import {useSettingsStore} from "../../stores/settings-store.ts";
import Button from "./Button.tsx";

function RadioButton() {
    const radioState = useRadioStore(state => state.radioState?.state ?? "NotConfigured");
    const disabled = radioState === "NotConfigured" || radioState === "Disconnected";
    const textMuted = radioState === "NotConfigured";
    const radioIntegration = useSettingsStore(state => state.radioConfig?.integration);

    const collapsed = useProfileType() === "tabbed";

    const buttonColor = () => {
        switch (radioState) {
            case "NotConfigured":
            case "Disconnected":
                return "gray";
            case "Connected":
            case "VoiceConnected":
                return "gray";
            case "RxIdle":
                return "emerald";
            case "RxActive":
                return "cornflower";
            case "TxActive":
                return "cornflower";
            case "Error":
                return "red";
            default:
                return "gray";
        }
    };

    const handleButtonClick = () => {
        if (!disabled && radioIntegration === "TrackAudio") {
            goToPage("radio");
        }

        if (
            radioState !== "NotConfigured" &&
            (radioState === "Disconnected" || radioState === "Error")
        ) {
            void invokeStrict("keybinds_reconnect_radio");
        }
    };

    return (
        <Button
            color={buttonColor()}
            disabled={radioState === "NotConfigured"}
            softDisabled={disabled}
            onClick={handleButtonClick}
            className={clsx(
                "text-lg transition-[width]",
                textMuted && "text-gray-500",
                collapsed ? "w-24" : "w-46",
            )}
        >
            Radio
        </Button>
    );
}

export default RadioButton;
