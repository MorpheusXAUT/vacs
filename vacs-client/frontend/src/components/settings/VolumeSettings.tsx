import VolumeSlider from "./VolumeSlider.tsx";
import mic from "../../assets/mic.svg";
import headphones from "../../assets/headphones.svg";
import mousePointerClick from "../../assets/mouse-pointer-click.svg";
import bellRing from "../../assets/bell-ring.svg";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {invokeSafe, invokeStrict} from "../../error.ts";
import {useEffect, useState} from "preact/hooks";
import {AudioVolumes} from "../../types/audio.ts";
import InputLevelMeter from "./InputLevelMeter.tsx";

function VolumeSettings() {
    const [volumes, setVolumes] = useState<AudioVolumes>({
        input: 0.5,
        output: 0.5,
        click: 0.5,
        chime: 0.5,
    });

    const handleVolumeSave = useAsyncDebounce(async (type: keyof AudioVolumes, volume: number) => {
        await invokeSafe("audio_set_volume", {volumeType: type, volume: volume});
    });

    useEffect(() => {
        const fetchVolumes = async () => {
            try {
                const volumes = await invokeStrict<AudioVolumes>("audio_get_volumes");
                setVolumes(volumes);
            } catch {}
        };

        void fetchVolumes();
    }, []);

    return (
        <>
            <div className="h-full w-64 border-r-2 border-zinc-200 flex flex-col">
                <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">
                    Call
                </p>
                <div className="w-full grow px-3 py-1.5 flex flex-row gap-3.5">
                    <div className="w-full flex flex-col items-center">
                        <img src={headphones} className="pt-1 h-12 w-12" alt="" />
                        <p className="font-semibold text-center pt-3 pb-1">Output</p>
                        <VolumeSlider
                            position={volumes.output}
                            setPosition={position =>
                                setVolumes(prev => ({...prev, output: position}))
                            }
                            savePosition={position => handleVolumeSave("output", position)}
                        />
                    </div>
                    <div className="w-full flex flex-row items-center">
                        <div className="w-full h-full flex flex-col items-center">
                            <img src={mic} className="pt-1 h-12 w-12" alt="" />
                            <p className="font-semibold text-center pt-3 pb-1">Input</p>
                            <VolumeSlider
                                position={volumes.input}
                                setPosition={position =>
                                    setVolumes(prev => ({...prev, input: position}))
                                }
                                savePosition={position => handleVolumeSave("input", position)}
                            />
                        </div>
                        <InputLevelMeter />
                    </div>
                </div>
            </div>
            <div className="h-full w-60 border-r-2 border-zinc-200 flex flex-col">
                <p className="w-full text-center border-b-2 border-zinc-200 uppercase font-semibold">
                    Settings Touch Panel
                </p>
                <div className="w-full grow px-3 py-1.5 flex flex-row gap-3.5">
                    <div className="w-full flex flex-col items-center">
                        <img src={mousePointerClick} className="pt-1 h-12 w-12" alt="" />
                        <p className="font-semibold text-center pt-1 leading-4">
                            Click
                            <br />
                            Volume
                        </p>
                        <VolumeSlider
                            position={volumes.click}
                            setPosition={position =>
                                setVolumes(prev => ({...prev, click: position}))
                            }
                            savePosition={position => handleVolumeSave("click", position)}
                        />
                    </div>
                    <div className="w-full flex flex-col items-center">
                        <img src={bellRing} className="pt-1 h-12 w-12" alt="" />
                        <p className="font-semibold text-center pt-1 leading-4">
                            Chime
                            <br />
                            Volume
                        </p>
                        <VolumeSlider
                            position={volumes.chime}
                            setPosition={position =>
                                setVolumes(prev => ({...prev, chime: position}))
                            }
                            savePosition={position => handleVolumeSave("chime", position)}
                        />
                    </div>
                </div>
            </div>
        </>
    );
}

export default VolumeSettings;
