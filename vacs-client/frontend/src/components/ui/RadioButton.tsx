import Button from "./Button.tsx";
import {clsx} from "clsx";
import {useRadioState} from "../../hooks/radio-state-hook.ts";

function RadioButton() {
    const {state, handleButtonClick} = useRadioState();
    const disabled = state === "NotConfigured" || state === "Disconnected";
    const textMuted = state === "NotConfigured";

    const buttonColor = () => {
        switch (state) {
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

    // TODO: Error state
    // TODO: mission page

    return (
        <Button color={buttonColor()}
                disabled={state === "NotConfigured"}
                softDisabled={disabled}
                onClick={handleButtonClick}
                className={clsx("text-xl w-46", textMuted && "text-gray-500")}>
            Radio
        </Button>
    );
}

export default RadioButton;