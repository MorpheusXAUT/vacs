import Button from "../components/ui/Button.tsx";
import reload from "../assets/reload-cw.svg";
import {useProfileStore} from "../stores/profile-store.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {invokeStrict} from "../error.ts";
import {useConnectionStore} from "../stores/connection-store.ts";
import {navigate} from "wouter/use-browser-location";

function MissionPage() {
    const testProfilePath = useProfileStore(state => state.testProfilePath);
    const setTestProfilePath = useProfileStore(state => state.setTestProfilePath);
    const setConnectionState = useConnectionStore(state => state.setConnectionState);
    const resetProfileStore = useProfileStore(state => state.reset);

    const handleLoadTestProfileClick = useAsyncDebounce(async () => {
        try {
            const path = await invokeStrict<string>("app_load_test_profile");
            setTestProfilePath(path);
        } catch {}
    });

    const handleReloadClick = useAsyncDebounce(async () => {
        if (testProfilePath === undefined) return;
        await invokeStrict("app_load_test_profile", {path: testProfilePath});
    });

    const handleUnloadClick = () => {
        setConnectionState("disconnected");
        resetProfileStore();
        setTestProfilePath(undefined);
        navigate("/");
    };

    return (
        <div className="z-10 absolute h-[calc(100%+5rem+5rem+3px-0.5rem)] w-[calc(100%+3px)] translate-y-[calc(-4.75rem-1px)] translate-x-[calc(-1*(1px))] bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto rounded">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Mission</p>
            <div className="relative flex-1 min-h-0 flex flex-col rounded-b-sm bg-[#B5BBC6] ">
                <div className="w-full flex-1 min-h-0 flex justify-center items-center">
                    <p className="text-slate-600">Not implemented</p>
                </div>
                <div className="absolute right-0 bottom-0 border-t-2 border-l-2 border-white p-3 flex gap-3">
                    <Button
                        color="gray"
                        className="w-60 whitespace-nowrap px-3 py-2"
                        onClick={handleLoadTestProfileClick}
                    >
                        Load test profile
                    </Button>
                    <Button
                        color="gray"
                        className="w-[2.625rem] whitespace-nowrap flex justify-center items-center"
                        onClick={handleReloadClick}
                        disabled={testProfilePath === undefined}
                    >
                        <img src={reload} alt="Reload" />
                    </Button>
                    <Button
                        color="gray"
                        className="w-[2.625rem] whitespace-nowrap flex justify-center items-center"
                        onClick={handleUnloadClick}
                        disabled={testProfilePath === undefined}
                    >
                        <svg
                            width="26"
                            height="26"
                            viewBox="0 0 128 128"
                            fill="none"
                            className="w-full"
                            xmlns="http://www.w3.org/2000/svg"
                        >
                            <g clipPath="url(#clip0_0_1)">
                                <path d="M98 30L30 98" stroke="black" strokeWidth="12" />
                                <path d="M30 30L98 98" stroke="black" strokeWidth="12" />
                            </g>
                            <defs>
                                <clipPath id="clip0_0_1">
                                    <rect
                                        width="128"
                                        height="128"
                                        fill="white"
                                        transform="matrix(-1 0 0 1 128 0)"
                                    />
                                </clipPath>
                            </defs>
                        </svg>
                    </Button>
                </div>
            </div>
        </div>
    );
}

export default MissionPage;
