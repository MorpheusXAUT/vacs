import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {ClipMeta} from "../../types/replay.ts";
import {invokeSafe} from "../../error.ts";
import Button from "../ui/Button.tsx";
import {StateSetter} from "../../types/generic.ts";

type PlaybackActionsProps = {
    clips: ClipMeta[];
    selected: number;
    setClips: StateSetter<ClipMeta[]>;
    playing: boolean;
};

function PlaybackActions({clips, selected, setClips, playing}: PlaybackActionsProps) {
    const handleDelete = useAsyncDebounce(async (clip: ClipMeta) => {
        await invokeSafe("replay_delete", {id: clip.id});
        setClips(prev => prev.filter(c => c.id !== clip.id));
    });

    const handleExport = useAsyncDebounce(async (clip: ClipMeta) => {
        await invokeSafe("replay_export", {id: clip.id});
    });

    const handleClear = useAsyncDebounce(async () => {
        await invokeSafe("replay_clear");
        setClips([]);
    });

    return (
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
                disabled={clips[selected] === undefined || playing}
                onClick={() => handleDelete(clips[selected])}
            >
                Delete
            </Button>
            <Button
                color="gray"
                className="h-17 uppercase"
                disabled={clips.length === 0 || playing}
                onClick={handleClear}
            >
                Delete <br /> All
            </Button>
        </div>
    );
}

export default PlaybackActions;
