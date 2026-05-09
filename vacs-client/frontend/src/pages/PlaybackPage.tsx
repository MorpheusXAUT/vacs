import {clsx} from "clsx";
import {CloseButton} from "./SettingsPage.tsx";
import Button from "../components/ui/Button.tsx";
import PlaybackControls from "../components/playback/PlaybackControls.tsx";
import PlaybackProgress from "../components/playback/PlaybackProgress.tsx";
import PlaybackList from "../components/playback/PlaybackList.tsx";

function PlaybackPage() {
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
                        <Button color="gray" className="h-17 uppercase" disabled>
                            Export
                        </Button>
                        <Button color="gray" className="h-17 uppercase">
                            Delete
                        </Button>
                        <Button color="gray" className="h-17 uppercase">
                            Delete <br /> All
                        </Button>
                    </div>
                </div>
                <div className="h-full w-full flex flex-col p-px">
                    <PlaybackList />
                    <div className="relative w-full h-full flex flex-col items-center pr-16">
                        <PlaybackProgress />
                        <PlaybackControls />
                        <CloseButton className="h-17 w-19! absolute bottom-0 right-0" />
                    </div>
                </div>
            </div>
        </div>
    );
}

export default PlaybackPage;
