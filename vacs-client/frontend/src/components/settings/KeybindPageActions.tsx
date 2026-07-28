import {invokeSafe} from "../../error";
import {useCapabilitiesStore} from "../../stores/capabilities-store";
import {openSettingsSubmenu} from "../../stores/navigation-store";
import Button from "../ui/Button";

function KeybindPageActions() {
    const capPlatform = useCapabilitiesStore(state => state.platform);

    const handleOpenSystemShortcutsOnClick = async () => {
        void invokeSafe("audio_play_ui_click");
        await invokeSafe("keybinds_open_system_shortcuts_settings");
    };

    return (
        <div className="h-full flex items-center justify-center gap-2 [&_button]:h-full">
            <Button
                color="gray"
                className="w-22 text-sm"
                onClick={() => openSettingsSubmenu("settings-joystick-devices")}
            >
                <p>
                    Joystick
                    <br />
                    Devices
                </p>
            </Button>
            {capPlatform === "LinuxWayland" && (
                <Button
                    color="gray"
                    className="w-24 text-sm"
                    onClick={handleOpenSystemShortcutsOnClick}
                >
                    <p>
                        System
                        <br />
                        Shortcuts
                    </p>
                </Button>
            )}
        </div>
    );
}

export default KeybindPageActions;
