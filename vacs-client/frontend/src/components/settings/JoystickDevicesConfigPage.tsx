import {useEffect, useState} from "preact/hooks";
import {useNavigationStore} from "../../stores/navigation-store";
import {JoystickDeviceEntry} from "../../types/keybinds";
import Checkbox from "../ui/Checkbox";
import SettingsSubPage from "./SettingsSubPage";
import {invokeStrict} from "../../error";
import {useAsyncDebounce} from "../../hooks/debounce-hook";

function JoystickDevicesConfigPage() {
    const [joysticks, setJoysticks] = useState<JoystickDeviceEntry[] | undefined>(undefined);

    const prevSubmenu = useNavigationStore(state => state.previous.submenu);

    const handleOnIgnoreChange = useAsyncDebounce(async (device: string, ignored: boolean) => {
        if (joysticks === undefined) return;
        const joystick = joysticks.find(j => j.device === device);
        if (joystick === undefined) return;

        const next: JoystickDeviceEntry[] = [
            ...joysticks.filter(j => j.device !== device),
            {...joystick, ignored},
        ];

        setJoysticks(next);

        const ignoredDevices = next.filter(j => j.ignored);

        try {
            await invokeStrict("keybinds_set_ignored_joysticks", {devices: ignoredDevices});
        } catch {
            setJoysticks(joysticks);
        }
    });

    useEffect(() => {
        const fetchJoysticks = async () => {
            try {
                const joysticks = await invokeStrict<JoystickDeviceEntry[]>(
                    "keybinds_list_joystick_devices",
                );
                setJoysticks(joysticks);
            } catch {}
        };

        void fetchJoysticks();
    }, []);

    return (
        <SettingsSubPage
            title="Joystick Devices"
            className="max-h-[calc(100%-5rem)]"
            width="w-[50%]"
            closeTargetSubmenu={prevSubmenu}
        >
            <div className="h-full flex flex-col">
                <div className="w-full mb-1 flex flex-row gap-2 items-center justify-center border-b-2 border-zinc-200">
                    <p className="font-semibold uppercase">Ignored Joystick Devices</p>
                </div>
                <div className="w-full h-full flex flex-col gap-2 py-1 px-3 overflow-auto">
                    {joysticks !== undefined ? (
                        joysticks
                            .sort(sortJoysticks)
                            .map(joystick => (
                                <IgnoreJoystickEntry
                                    key={joystick.device}
                                    entry={joystick}
                                    onIgnoreChange={ignored =>
                                        handleOnIgnoreChange(joystick.device, ignored)
                                    }
                                />
                            ))
                    ) : (
                        <p className="self-center">Loading...</p>
                    )}
                </div>
            </div>
        </SettingsSubPage>
    );
}

function sortJoysticks(a: JoystickDeviceEntry, b: JoystickDeviceEntry) {
    if (a.name !== undefined && b.name !== undefined) {
        return a.name.localeCompare(b.name);
    } else if (a.name !== undefined) {
        return -1;
    } else if (b.name !== undefined) {
        return 1;
    }

    return a.device.localeCompare(b.device);
}

type IgnoreJoystickEntryProps = {
    entry: JoystickDeviceEntry;
    onIgnoreChange: (ignored: boolean) => void;
};

function IgnoreJoystickEntry(props: IgnoreJoystickEntryProps) {
    return (
        <div className="w-full flex flex-row gap-2 items-center">
            <Checkbox
                className="shrink-0"
                name={props.entry.device}
                checked={props.entry.ignored}
                onChange={e => props.onIgnoreChange(e.currentTarget.checked)}
            />
            <label
                for={props.entry.device}
                className="max-w-full truncate"
                title={props.entry.name ?? props.entry.device}
            >
                {props.entry.name ?? props.entry.device}
            </label>
        </div>
    );
}

export default JoystickDevicesConfigPage;
