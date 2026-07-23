import {clsx} from "clsx";
import {useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../../error.ts";
import {useCapabilitiesStore} from "../../stores/capabilities-store.ts";
import {KeybindType} from "../../types/keybinds.ts";
import {InputBinding, isJoystickButton} from "../../types/transmit.ts";
import KeyCapture from "./KeyCapture.tsx";

type CombinedKeybindFieldProps = {
    type: KeybindType | null;
    binding: InputBinding | null;
    bindingLabel: string | null;
    className?: string;
    disabled?: boolean;
    onCapture: (input: InputBinding) => Promise<void>;
    onRemove: () => Promise<void>;
};

// Wayland keybind field: keyboard shortcuts are managed by the desktop
// environment (XDG portal) and can only be inspected here, while joystick
// buttons are captured in-app. A bound joystick button replaces the portal
// shortcut for the action, so the field always shows the active trigger;
// removing the button falls back to the portal shortcut.
function CombinedKeybindField({
    type,
    binding,
    bindingLabel,
    className,
    disabled = false,
    onCapture,
    onRemove,
}: CombinedKeybindFieldProps) {
    const capJoystick = useCapabilitiesStore(state => state.joystick);
    const [externalBinding, setExternalBinding] = useState<string | null>(null);
    const hasJoystickBinding = isJoystickButton(binding);

    useEffect(() => {
        const fetchExternalBinding = async () => {
            if (type === null) {
                setExternalBinding(null);
                return;
            }

            try {
                const binding = await invokeStrict<string | null>("keybinds_get_external_binding", {
                    keybind: type,
                });
                setExternalBinding(binding);
            } catch {}
        };

        void fetchExternalBinding();
    }, [type]);

    return (
        <KeyCapture
            label={hasJoystickBinding ? bindingLabel : (externalBinding ?? null)}
            className={clsx(!hasJoystickBinding && "text-gray-500", className)}
            keyboardEnabled={false}
            disabled={disabled || !capJoystick}
            removeDisabled={!hasJoystickBinding}
            onCapture={onCapture}
            onRemove={onRemove}
        />
    );
}

export default CombinedKeybindField;
