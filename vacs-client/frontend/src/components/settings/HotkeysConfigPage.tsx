import KeyCapture from "./KeyCapture.tsx";
import {codeToLabel} from "../../types/transmit.ts";
import {useEffect, useState} from "preact/hooks";
import {KeybindsConfig, KeybindType} from "../../types/keybinds.ts";
import {invokeStrict} from "../../error.ts";
import {useCapabilitiesStore} from "../../stores/capabilities-store.ts";
import SettingsSubPage from "./SettingsSubPage.tsx";
import ExternalKeybindField from "./ExternalKeybindField.tsx";

type Keybind = {
    code: string | null;
    label: string | null;
};

async function codeToKeybind(code: string | null): Promise<Keybind> {
    return {code, label: code && (await codeToLabel(code))};
}

function HotkeysConfigPage() {
    const [acceptCall, setAcceptCall] = useState<Keybind | undefined>(undefined);
    const [endCall, setEndCall] = useState<Keybind | undefined>(undefined);
    const [toggleRadioPrio, setToggleRadioPrio] = useState<Keybind | undefined>(undefined);

    useEffect(() => {
        const fetchConfig = async () => {
            try {
                const config = await invokeStrict<KeybindsConfig>("keybinds_get_keybinds_config");
                setAcceptCall(await codeToKeybind(config.acceptCall));
                setEndCall(await codeToKeybind(config.endCall));
                setToggleRadioPrio(await codeToKeybind(config.toggleRadioPrio));
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

    const handleOnCapture = async (code: string | null) => {
        try {
            await invokeStrict("keybinds_set_binding", {keybind: type, code});
            setKeybind(await codeToKeybind(code));
        } catch {}
    };

    return (
        <>
            <p>{label}</p>
            {hasExternal ? (
                <ExternalKeybindField type={type} />
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
