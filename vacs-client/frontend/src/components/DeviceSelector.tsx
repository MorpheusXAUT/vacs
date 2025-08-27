import Select, {SelectOption} from "./ui/Select.tsx";
import {useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../error.ts";
import {AudioDevices} from "../types/audio.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {useCallStore} from "../stores/call-store.ts";

type DeviceSelectorProps = {
    deviceType: "Input" | "Output";
}

function DeviceSelector(props: DeviceSelectorProps) {
    const callDisplayType = useCallStore(state => state.callDisplay?.type);

    const [device, setDevice] = useState<string>("");
    const [devices, setDevices] = useState<SelectOption[]>([{value: "", text: "Loading..."}]);

    const handleOnChange = useAsyncDebounce(async (new_device: string) => {
        const previousDeviceName = device;

        setDevice(new_device);

        try {
            await invokeStrict("audio_set_device", {deviceType: props.deviceType, deviceName: new_device});
        } catch {
            setDevice(previousDeviceName);
        }
    });

    useEffect(() => {
        const fetchDevices = async () => {
            try {
                const audioDevices = await invokeStrict<AudioDevices>("audio_get_devices", {
                    deviceType: props.deviceType
                });

                const defaultDevice = {
                    value: "", text: `Default (${audioDevices.default})`
                };

                let deviceList = audioDevices.all.map((deviceName) => ({value: deviceName, text: deviceName}));
                deviceList = [defaultDevice, ...deviceList];
                if (audioDevices.preferred.length !== 0 && audioDevices.preferred !== audioDevices.picked) {
                    // TODO add colors to preferred (red) and picked (green) devices
                    deviceList.push({value: audioDevices.preferred, text: audioDevices.preferred});
                }

                setDevice(audioDevices.preferred);
                setDevices(deviceList);
            } catch {
            }
        };

        void fetchDevices();
    }, []);

    return (
        <>
            <p className="w-full text-center font-semibold">{props.deviceType === "Output" ? "Headset" : "Microphone"}</p>
            <Select
                options={devices}
                selected={device}
                onChange={handleOnChange}
                disabled={devices === undefined || devices.length === 0 || callDisplayType === "accepted" || callDisplayType === "outgoing"}
            />
        </>
    );
}

export default DeviceSelector;