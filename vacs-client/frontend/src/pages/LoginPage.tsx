import {invokeSafe} from "../error.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {clsx} from "clsx";
import {useEffect, useState} from "preact/hooks";
import {listen} from "@tauri-apps/api/event";

function LoginPage() {
    const [loading, setLoading] = useState<boolean>(false);

    const handleLoginClick = useAsyncDebounce(async () => {
        setLoading(true);
        void invokeSafe("audio_play_ui_click");
        await invokeSafe("auth_open_oauth_url");
    });

    useEffect(() => {
        const unlisten = listen("auth:error", () => {
            setLoading(false);
        });

        return () => unlisten.then(fn => fn());
    }, []);

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <button
                className={clsx(
                    "w-54 px-3 py-2 border-2 text-amber-50 rounded cursor-pointer disabled:cursor-not-allowed text-lg",
                    "border-t-[#98C9EC] border-l-[#98C9EC] border-r-[#15603D] border-b-[#15603D] shadow-[0_0_0_1px_#579595]",
                    "active:enabled:border-b-[#98C9EC] active:enabled:border-r-[#98C9EC] active:enabled:border-l-[#15603D] active:enabled:border-t-[#15603D]",
                    "disabled:brightness-90",
                )}
                style={{background: "linear-gradient(to bottom right, #2483C5 0%, #29B473 100%) border-box"}}
                onClick={handleLoginClick}
                disabled={loading}
            >
                {!loading ? "Login via VATSIM" : "Loading..."}
            </button>
        </div>
    );
}

export default LoginPage;