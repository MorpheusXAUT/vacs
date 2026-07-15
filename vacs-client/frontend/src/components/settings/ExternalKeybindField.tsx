import {KeybindType} from "../../types/keybinds.ts";
import {useEffect, useState} from "preact/hooks";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe, invokeStrict} from "../../error.ts";
import {clsx} from "clsx";

type ExternalKeybindFieldProps = {type: KeybindType | null; className?: string; disabled?: boolean};

function ExternalKeybindField({type, className, disabled = false}: ExternalKeybindFieldProps) {
    const [binding, setBinding] = useState<string | null | undefined>(undefined);

    const handleOpenSystemShortcutsOnClick = useAsyncDebounce(async () => {
        if (type === null || disabled) return;

        await invokeSafe("keybinds_open_system_shortcuts_settings");
    });

    useEffect(() => {
        const fetchExternalBinding = async () => {
            if (type === null) return;

            try {
                const binding = await invokeStrict<string | null>("keybinds_get_external_binding", {
                    keybind: type,
                });
                setBinding(binding);
            } catch {}
        };

        void fetchExternalBinding();
    }, [type]);

    return (
        <div
            onClick={handleOpenSystemShortcutsOnClick}
            title={
                type === null || disabled
                    ? ""
                    : "On Wayland, shortcuts are managed by the system. Please configure the shortcut in your desktop environment settings. Click this field to try opening the appropriate system settings."
            }
            className={clsx(
                "w-full h-full min-w-10 min-h-8 grow text-sm py-1 px-2 rounded text-center flex items-center justify-center",
                "bg-gray-300 border-2 border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700",
                "brightness-90",
                type === null || disabled ? "cursor-not-allowed" : "cursor-help",
                className,
            )}
        >
            <p className="truncate max-w-full">{type === null ? "" : binding || "Not bound"}</p>
        </div>
    );
}

export default ExternalKeybindField;
