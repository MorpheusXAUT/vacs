import Select, {SelectOption} from "../ui/Select.tsx";
import {useCallback, useEffect, useState} from "preact/hooks";
import {invokeStrict} from "../../error.ts";
import {AudioDevices} from "../../types/audio.ts";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {useCallStore} from "../../stores/call-store.ts";
import {clsx} from "clsx";

type DeviceSelectorProps = {
    deviceType: "Input" | "Output";
};

function DeviceSelector(props: DeviceSelectorProps) {
    const [device, setDevice] = useState<string>("");
    const [isFallback, setIsFallback] = useState<boolean>(false);
    const [devices, setDevices] = useState<SelectOption[]>([{value: "", text: "Loading..."}]);

    const callDisplayType = useCallStore(state => state.callDisplay?.type);

    const fetchDevices = useCallback(async () => {
        try {
            const audioDevices = await invokeStrict<AudioDevices>("audio_get_devices", {
                deviceType: props.deviceType,
            });

            const isFallback =
                audioDevices.preferred.length !== 0 &&
                audioDevices.preferred !== audioDevices.picked;
            const defaultDevice = {
                value: "",
                text: `Default (${audioDevices.default})`,
                className: "text-initial",
            };

            let deviceList: SelectOption[] = audioDevices.all.map(deviceName => ({
                value: deviceName,
                text: deviceName,
                className:
                    isFallback && deviceName === audioDevices.picked
                        ? "text-green-700"
                        : "text-initial",
            }));

            deviceList = [defaultDevice, ...deviceList];
            if (isFallback) {
                deviceList.push({
                    value: audioDevices.preferred,
                    text: audioDevices.preferred,
                    hidden: true,
                    disabled: true,
                });
            }

            setIsFallback(isFallback);
            setDevice(audioDevices.preferred);
            setDevices(deviceList);
        } catch {}
    }, [props.deviceType]);

    const handleOnChange = useAsyncDebounce(async (new_device: string) => {
        const previousDeviceName = device;

        setDevice(new_device);

        try {
            await invokeStrict("audio_set_device", {
                deviceType: props.deviceType,
                deviceName: new_device,
            });
            await fetchDevices();
        } catch {
            setDevice(previousDeviceName);
        }
    });

    useEffect(() => {
        void fetchDevices();
    }, [fetchDevices]);

    return (
        <>
            <p className="w-full text-center font-semibold">
                {props.deviceType === "Output" ? "Headset" : "Microphone"}
            </p>
            <Select
                name={props.deviceType}
                className={clsx("mb-1", isFallback && "text-red-500 disabled:!text-[#B34F5C]")}
                options={devices}
                selected={device}
                onChange={handleOnChange}
                disabled={
                    devices === undefined ||
                    devices.length === 0 ||
                    callDisplayType === "accepted" ||
                    callDisplayType === "outgoing"
                }
            />
        </>
    );
}

export default DeviceSelector;
