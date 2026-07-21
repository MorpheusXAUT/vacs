import {clsx} from "clsx";
import {useEffect, useState} from "preact/hooks";
import {invokeSafe, invokeStrict} from "../../error.ts";
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

    const handleOpenSystemShortcutsOnClick = async () => {
        if (disabled) return;

        void invokeSafe("audio_play_ui_click");
        await invokeSafe("keybinds_open_system_shortcuts_settings");
    };

    return (
        <div className="grow h-full min-w-0 flex flex-row items-center justify-center">
            <KeyCapture
                label={hasJoystickBinding ? bindingLabel : (externalBinding ?? null)}
                className={clsx(!hasJoystickBinding && "text-gray-500", className)}
                keyboardEnabled={false}
                disabled={disabled || !capJoystick}
                removeDisabled={!hasJoystickBinding}
                onCapture={onCapture}
                onRemove={onRemove}
            />
            <svg
                onClick={handleOpenSystemShortcutsOnClick}
                xmlns="http://www.w3.org/2000/svg"
                width="27"
                height="27"
                viewBox="0 0 24 24"
                fill="none"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className={clsx(
                    "shrink-0 p-1 pr-0!",
                    disabled
                        ? "stroke-gray-500 cursor-not-allowed"
                        : "stroke-gray-700 hover:stroke-blue-500 transition-colors cursor-pointer",
                )}
            >
                <title>
                    On Wayland, keyboard shortcuts are managed by the system. Click to open your
                    desktop environment's shortcut settings. Binding a joystick button here replaces
                    the system shortcut for this action.
                </title>
                <rect x="2" y="6" width="20" height="12" rx="2" />
                <path d="M6 10h0M10 10h0M14 10h0M18 10h0M6 14h0M18 14h0M10 14h4" />
            </svg>
        </div>
    );
}

export default CombinedKeybindField;
