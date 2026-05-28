import {clsx} from "clsx";
import {useEffect, useRef, useState} from "preact/hooks";
import PlaybackActions from "../components/playback/PlaybackActions.tsx";
import PlaybackControls from "../components/playback/PlaybackControls.tsx";
import PlaybackList from "../components/playback/PlaybackList.tsx";
import Button from "../components/ui/Button.tsx";
import {invokeSafe} from "../error.ts";
import {useCapabilitiesStore} from "../stores/capabilities-store.ts";
import {openSettingsSubmenu} from "../stores/navigation-store.ts";
import {useSettingsStore} from "../stores/settings-store.ts";
import {listen, UnlistenFn} from "../transport";
import {ClipMeta, sortClips} from "../types/replay.ts";
import {CloseButton} from "./SettingsPage.tsx";

function PlaybackPage() {
    const capReplay = useCapabilitiesStore(state => state.replay);
    const capPlatform = useCapabilitiesStore(state => state.platform);

    const replayEnabled = useSettingsStore(state => state.replayEnabled);

    return (
        <div
            className={clsx(
                "z-10 absolute h-[calc(100%+3px)] w-[44rem] -top-px right-[-2px]",
                "bg-blue-700 px-2 pb-2 flex flex-col rounded-md",
            )}
        >
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Playback</p>
            {capReplay && replayEnabled ? (
                <PlaybackPageInner />
            ) : !replayEnabled ? (
                <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col justify-center items-center text-slate-600">
                    <p>Radio playback is not enabled.</p>
                    <p>
                        Enable it in the{" "}
                        <span
                            className="text-blue-700 cursor-pointer"
                            onClick={() => {
                                void invokeSafe("audio_play_ui_click");
                                openSettingsSubmenu("settings-advanced");
                            }}
                        >
                            advanced settings
                        </span>
                        .
                    </p>
                </div>
            ) : (
                <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex justify-center items-center text-slate-600">
                    Radio playback is not yet supported on {capPlatform}.
                </div>
            )}
        </div>
    );
}

function PlaybackPageInner() {
    const [playing, setPlaying] = useState(false);
    const playingRef = useRef(playing);

    const [clips, setClips] = useState<ClipMeta[]>([]);
    const [selected, setSelected] = useState<number>(0);

    useEffect(() => {
        playingRef.current = playing;
    }, [playing]);

    useEffect(() => {
        const fetch = async () => {
            const list = await invokeSafe<ClipMeta[]>("replay_list");
            if (list === undefined) return;
            setClips(sortClips(list));
        };
        void fetch();

        const unlistenFns: Promise<UnlistenFn>[] = [];
        unlistenFns.push(
            listen<ClipMeta>("replay:clip-recorded", event => {
                setClips(prev => {
                    if (prev.length > 0 && playingRef.current) setSelected(prev => prev + 1);
                    return sortClips([...prev, event.payload]);
                });
            }),
            listen<ClipMeta>("replay:clip-evicted", event => {
                setClips(prev => prev.filter(c => c.id !== event.payload.id));
                // TODO: stop playback if playing and this clip is selected
            }),
        );

        return () => {
            unlistenFns.forEach(fn => fn.then(f => f()));
        };
    }, []);

    return (
        <div className="w-full grow rounded-b-sm bg-[#B5BBC6] grid grid-cols-[6.5rem_auto] p-2 gap-2 overflow-auto">
            <div className="h-full w-full flex flex-col justify-between items-center">
                <div className="w-full flex flex-col items-center bg-gray-300 border rounded-md">
                    <p className="w-full border-b text-center font-semibold">Filter</p>
                    <Button color="gray" className="h-15 my-2 uppercase">
                        <p>
                            Speech <br /> Only
                        </p>
                    </Button>
                    <Button color="blue" className="h-15 mt-2 uppercase rounded-b-none!">
                        Radio
                    </Button>
                    <Button color="gray" className="h-15 mb-2 uppercase rounded-t-none!">
                        Phone
                    </Button>
                </div>
                <PlaybackActions
                    clips={clips}
                    selected={selected}
                    setClips={setClips}
                    playing={playing}
                />
            </div>
            <div className="h-full w-full flex flex-col p-px">
                <PlaybackList clips={clips} selected={selected} setSelected={setSelected} />
                <div className="relative w-full h-full flex flex-col items-center pr-16">
                    <PlaybackControls
                        playing={playing}
                        playingRef={playingRef}
                        setPlaying={setPlaying}
                        clip={clips[selected]}
                        prevClip={clips[selected + 1]}
                        nextClip={clips[selected - 1]}
                        setSelectedClip={setSelected}
                    />
                    <CloseButton className="h-17 w-19! absolute bottom-0 right-0" />
                </div>
            </div>
        </div>
    );
}

export default PlaybackPage;
