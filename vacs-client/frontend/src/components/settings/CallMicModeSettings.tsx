import {invokeStrict} from "../../error.ts";
import {useCapabilitiesStore} from "../../stores/capabilities-store.ts";
import {callMicModeToKeybind} from "../../types/keybinds.ts";
import {
    InputBinding,
    TransmitConfig,
    TransmitConfigWithLabels,
    inputEquals,
    isCallMicMode,
    isJoystickButton,
    withTransmitLabels,
} from "../../types/transmit.ts";
import Select from "../ui/Select.tsx";
import ExternalKeybindField from "./ExternalKeybindField.tsx";
import KeyCapture from "./KeyCapture.tsx";

type CallMicModeSettingsProps = {
    transmitConfig: TransmitConfigWithLabels;
    setTransmitConfig: (config: TransmitConfigWithLabels) => void;
};

function CallMicModeSettings({transmitConfig, setTransmitConfig}: CallMicModeSettingsProps) {
    const capPlatform = useCapabilitiesStore(state => state.platform);
    const capJoystick = useCapabilitiesStore(state => state.joystick);

    const handleOnTransmitCapture = async (input: InputBinding) => {
        if (transmitConfig.callMicMode === "VoiceActivation") return;

        let newConfig: TransmitConfig;
        switch (transmitConfig.callMicMode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: input};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: input};
                break;
        }

        if (inputEquals(input, transmitConfig.radioPushToTalk)) {
            newConfig.radioPushToTalk = null;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const handleOnTransmitModeChange = async (value: string) => {
        if (!isCallMicMode(value)) return;

        const previousTransmitConfig = transmitConfig;
        const newTransmitConfig: TransmitConfigWithLabels = {...transmitConfig, callMicMode: value};

        setTransmitConfig(newTransmitConfig);

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newTransmitConfig});
        } catch {
            setTransmitConfig(previousTransmitConfig);
        }
    };

    const handleOnTransmitRemoveClick = async () => {
        if (transmitConfig.callMicMode === "VoiceActivation") return;

        let newConfig: TransmitConfig;
        switch (transmitConfig.callMicMode) {
            case "PushToTalk":
                newConfig = {...transmitConfig, pushToTalk: null};
                break;
            case "PushToMute":
                newConfig = {...transmitConfig, pushToMute: null};
                break;
        }

        try {
            await invokeStrict("keybinds_set_transmit_config", {transmitConfig: newConfig});
            setTransmitConfig(await withTransmitLabels(newConfig));
        } catch {}
    };

    const activeCallBinding =
        transmitConfig.callMicMode === "PushToTalk"
            ? transmitConfig.pushToTalk
            : transmitConfig.callMicMode === "PushToMute"
              ? transmitConfig.pushToMute
              : null;

    const activeCallLabel =
        transmitConfig.callMicMode === "PushToTalk"
            ? transmitConfig.pushToTalkLabel
            : transmitConfig.callMicMode === "PushToMute"
              ? transmitConfig.pushToMuteLabel
              : "";

    return (
        <>
            <Select
                className="w-[21ch]! h-full"
                name="keybind-mode"
                options={[
                    {value: "VoiceActivation", text: "Voice activation"},
                    {value: "PushToTalk", text: "Push-to-talk"},
                    {value: "PushToMute", text: "Push-to-mute"},
                ]}
                selected={transmitConfig.callMicMode}
                onChange={handleOnTransmitModeChange}
            />
            {capPlatform === "LinuxWayland" ? (
                // The keyboard shortcut is managed by the desktop environment,
                // while a joystick button can be bound in parallel (joystick
                // input bypasses the portal).
                <>
                    <ExternalKeybindField type={callMicModeToKeybind(transmitConfig.callMicMode)} />
                    {capJoystick && (
                        <KeyCapture
                            label={isJoystickButton(activeCallBinding) ? activeCallLabel : null}
                            keyboardEnabled={false}
                            disabled={transmitConfig.callMicMode === "VoiceActivation"}
                            onCapture={handleOnTransmitCapture}
                            onRemove={handleOnTransmitRemoveClick}
                        />
                    )}
                </>
            ) : (
                <KeyCapture
                    label={activeCallLabel}
                    onCapture={handleOnTransmitCapture}
                    onRemove={handleOnTransmitRemoveClick}
                    disabled={transmitConfig.callMicMode === "VoiceActivation"}
                />
            )}
        </>
    );
}

export default CallMicModeSettings;
