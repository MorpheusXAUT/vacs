import {clsx} from "clsx";
import {CloseButton} from "./SettingsPage.tsx";
import Button from "../components/ui/Button.tsx";
import PlaybackControls from "../components/playback/PlaybackControls.tsx";
import PlaybackList from "../components/playback/PlaybackList.tsx";
import {useEffect, useState} from "preact/hooks";
import {ClipMeta, clipUnixMs} from "../types/replay.ts";
import {invokeSafe} from "../error.ts";
import {listen, UnlistenFn} from "../transport";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";

function PlaybackPage() {
    const [clips, setClips] = useState<ClipMeta[]>([]);
    const [selected, setSelected] = useState<number>(0);

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
                    if (prev.length > 0) setSelected(prev => prev + 1);
                    return sortClips([...prev, event.payload]);
                });
            }),
            listen<ClipMeta>("replay:clip-evicted", event => {
                setClips(prev => prev.filter(c => c.id !== event.payload.id)); // TODO: move selected
            }),
        );

        return () => {
            unlistenFns.forEach(fn => fn.then(f => f()));
        };
    }, []);

    const handleDelete = useAsyncDebounce(async (clip: ClipMeta) => {
        await invokeSafe("replay_delete", {id: clip.id});
        setClips(prev => prev.filter(c => c.id !== clip.id));
        if (clips[selected].id === clip.id) setSelected(0);
    });

    const handleExport = useAsyncDebounce(async (clip: ClipMeta) => {
        await invokeSafe("replay_export", {id: clip.id});
    });

    const handleClear = useAsyncDebounce(async () => {
        await invokeSafe("replay_clear");
        setClips([]);
        setSelected(0);
    });

    return (
        <div
            className={clsx(
                "z-10 absolute h-[calc(100%+3px)] w-[44rem] -top-px right-[-2px]",
                "bg-blue-700 px-2 pb-2 flex flex-col rounded-md",
            )}
        >
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Playback</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] grid grid-cols-[6.5rem_auto] p-2 gap-2 overflow-auto">
                <div className="h-full w-full flex flex-col justify-between items-center">
                    <div className="w-full flex flex-col items-center bg-gray-300 border rounded-md">
                        <p className="w-full border-b text-center font-semibold">Filter</p>
                        <Button color="gray" className="h-15 my-2 uppercase">
                            Speech <br /> Only
                        </Button>
                        <Button color="blue" className="h-15 mt-2 uppercase rounded-b-none!">
                            Radio
                        </Button>
                        <Button color="blue" className="h-15 mb-2 uppercase rounded-t-none!">
                            Phone
                        </Button>
                    </div>
                    <div className="flex flex-col gap-3">
                        <Button
                            color="gray"
                            className="h-17 uppercase"
                            disabled={clips[selected] === undefined}
                            onClick={() => handleExport(clips[selected])}
                        >
                            Export
                        </Button>
                        <Button
                            color="gray"
                            className="h-17 uppercase"
                            disabled={clips[selected] === undefined}
                            onClick={() => handleDelete(clips[selected])}
                        >
                            Delete
                        </Button>
                        <Button
                            color="gray"
                            className="h-17 uppercase"
                            disabled={clips.length === 0}
                            onClick={handleClear}
                        >
                            Delete <br /> All
                        </Button>
                    </div>
                </div>
                <div className="h-full w-full flex flex-col p-px">
                    <PlaybackList clips={clips} selected={selected} setSelected={setSelected} />
                    <div className="relative w-full h-full flex flex-col items-center pr-16">
                        <PlaybackControls clip={clips[selected]} />
                        <CloseButton className="h-17 w-19! absolute bottom-0 right-0" />
                    </div>
                </div>
            </div>
        </div>
    );
}

function sortClips(list: ClipMeta[]): ClipMeta[] {
    return [...list].sort((a, b) => clipUnixMs(b.started_at) - clipUnixMs(a.started_at));
}

export default PlaybackPage;
