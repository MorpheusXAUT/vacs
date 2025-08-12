import Select, {SelectOption} from "./ui/Select.tsx";
import {useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../error.ts";
import {AudioDevices} from "../types/device.ts";

type DeviceSelectorProps = {
    deviceType: "Input" | "Output";
}

function DeviceSelector(props: DeviceSelectorProps) {
    const [device, setDevice] = useState<string>("");
    const [devices, setDevices] = useState<SelectOption[]>([{value: "", text: "Loading..."}]);

    const handleOnChange = async (device: string) => {
        try {
            await invokeStrict("audio_set_device", {deviceType: props.deviceType, deviceName: device});
            setDevice(device);
        } catch {
        }
    };

    useEffect(() => {
        const fetchDevices = async () => {
            try {
                const audioDevices = await invokeStrict<AudioDevices>("audio_get_devices", {
                    deviceType: props.deviceType
                });

                const defaultDevice = {
                    value: "", text: `Default (${audioDevices.default})`
                };

                const deviceList = audioDevices.all.map((deviceName) => ({value: deviceName, text: deviceName}));

                setDevice(audioDevices.selected);
                setDevices(() => [defaultDevice, ...deviceList]);
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
                disabled={devices === undefined || devices.length === 0}
            />
        </>
    );
}

export default DeviceSelector;