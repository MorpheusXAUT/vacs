import KeyCapture from "./KeyCapture.tsx";
import {InputBinding, inputToLabel, isJoystickButton} from "../../types/transmit.ts";
import {useEffect, useState} from "preact/hooks";
import {KeybindsConfig, KeybindType} from "../../types/keybinds.ts";
import {invokeStrict} from "../../error.ts";
import {useCapabilitiesStore} from "../../stores/capabilities-store.ts";
import SettingsSubPage from "./SettingsSubPage.tsx";
import ExternalKeybindField from "./ExternalKeybindField.tsx";

type Keybind = {
    input: InputBinding | null;
    label: string | null;
};

async function inputToKeybind(input: InputBinding | null): Promise<Keybind> {
    return {input, label: input && (await inputToLabel(input))};
}

function HotkeysConfigPage() {
    const [acceptCall, setAcceptCall] = useState<Keybind | undefined>(undefined);
    const [endCall, setEndCall] = useState<Keybind | undefined>(undefined);
    const [toggleRadioPrio, setToggleRadioPrio] = useState<Keybind | undefined>(undefined);

    useEffect(() => {
        const fetchConfig = async () => {
            try {
                const config = await invokeStrict<KeybindsConfig>("keybinds_get_keybinds_config");
                setAcceptCall(await inputToKeybind(config.acceptCall));
                setEndCall(await inputToKeybind(config.endCall));
                setToggleRadioPrio(await inputToKeybind(config.toggleRadioPrio));
            } catch {}
        };

        void fetchConfig();
    }, []);

    return (
        <SettingsSubPage title="Hotkeys Config" width="w-1/2" className="py-3 px-4">
            <div className="grid grid-cols-[auto_1fr] gap-4 items-center">
                <KeybindField
                    type="AcceptCall"
                    label="Accept first call"
                    keybind={acceptCall}
                    setKeybind={setAcceptCall}
                />
                <KeybindField
                    type="EndCall"
                    label="End active call"
                    keybind={endCall}
                    setKeybind={setEndCall}
                />
                <KeybindField
                    type="ToggleRadioPrio"
                    label="Toggle RADIO PRIO"
                    keybind={toggleRadioPrio}
                    setKeybind={setToggleRadioPrio}
                />
            </div>
        </SettingsSubPage>
    );
}

type KeybindFieldProps = {
    type: KeybindType;
    label: string;
    keybind?: Keybind;
    setKeybind: (keybind: Keybind) => void;
};

function KeybindField({type, label, keybind, setKeybind}: KeybindFieldProps) {
    const hasExternal = useCapabilitiesStore(state => state.platform === "LinuxWayland");
    const capJoystick = useCapabilitiesStore(state => state.joystick);

    const handleOnCapture = async (input: InputBinding | null) => {
        try {
            await invokeStrict("keybinds_set_binding", {keybind: type, input});
            setKeybind(await inputToKeybind(input));
        } catch {}
    };

    return (
        <>
            <p>{label}</p>
            {hasExternal ? (
                // On Wayland the keyboard shortcut is managed by the desktop
                // environment, while a joystick button can be bound in parallel
                // (joystick input bypasses the portal).
                <div className="flex flex-row items-center gap-2 min-w-0">
                    <ExternalKeybindField type={type} />
                    {capJoystick && (
                        <KeyCapture
                            label={
                                keybind !== undefined && isJoystickButton(keybind.input)
                                    ? keybind.label
                                    : null
                            }
                            keyboardEnabled={false}
                            onCapture={handleOnCapture}
                            onRemove={() => handleOnCapture(null)}
                        />
                    )}
                </div>
            ) : keybind !== undefined ? (
                <KeyCapture
                    label={keybind.label}
                    onCapture={handleOnCapture}
                    onRemove={() => handleOnCapture(null)}
                />
            ) : (
                <p>Loading...</p>
            )}
        </>
    );
}

export default HotkeysConfigPage;
