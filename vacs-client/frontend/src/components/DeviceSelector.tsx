import Select from "./ui/Select.tsx";
import {useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../error.ts";
import {Device} from "../types/device.ts";

type DeviceSelectorProps = {
    deviceType: "Input" | "Output";
}

function DeviceSelector(props: DeviceSelectorProps) {
    const [device, setDevice] = useState<string>("");
    const [devices, setDevices] = useState<Device[]>([]);

    const handleOnChange = (device: string) => {
        // TODO add proper default device handling
        // TODO call command
        setDevice(device);
    };

    useEffect(() => {
        const fetchDevices = async () => {
            try {
                const devices = (await invokeStrict<Device[]>("audio_get_devices", {
                    deviceType: props.deviceType
                })).sort((a, b) => Number(b.isDefault) - Number(a.isDefault));
                setDevices(devices);
                setDevice(devices[0].name);
            } catch {
            }
        };

        void fetchDevices();
    }, []);

    return (
        <>
            <p className="w-full text-center font-semibold">{props.deviceType === "Output" ? "Headset" : "Microphone"}</p>
            <Select
                options={devices.map((device) => ({
                    value: device.name,
                    text: `${device.isDefault ? "Default (" : ""}${device.name}${device.isDefault ? ")" : ""}`, // TODO make me proper code
                }))}
                selected={device} onChange={handleOnChange} disabled={devices.length === 0}/>
        </>
    );
}

export default DeviceSelector;