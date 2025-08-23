import {invokeSafe} from "../error.ts";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";

// TODO: Display state when logging in

function LoginPage() {
    const handleLoginClick = useAsyncDebounce(async () => {
        await invokeSafe("auth_open_oauth_url");
    });

    return (
        <div className="h-full w-full flex justify-center items-center p-4">
            <button
                className="px-6 py-2 border-2 text-amber-50 rounded cursor-pointer text-lg
                           border-t-[#98C9EC] border-l-[#98C9EC] border-r-[#15603D] border-b-[#15603D] shadow-[0_0_0_1px_#579595]
                           active:border-b-[#98C9EC] active:border-r-[#98C9EC] active:border-l-[#15603D] active:border-t-[#15603D]"
                style={{background: "linear-gradient(to bottom right, #2483C5 0%, #29B473 100%) border-box"}}
                onClick={handleLoginClick}
            >
                Login via VATSIM
            </button>
        </div>
    );
}

export default LoginPage;